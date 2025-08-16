use std::{
    collections::HashMap,
    fs,
    future::Future,
    path::Path,
    pin::Pin,
    process,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, LazyLock, Mutex,
    },
    time::Duration,
};
use anyhow::Error;
use axum::body::BodyDataStream;
use chrono::{DateTime, TimeDelta, Utc};
use sea_orm::ActiveValue::Set;
use tokio::{fs::File, time};
use uuid::Uuid;
use crate::config;

mod file;
mod metadata;

#[derive(Debug)]
pub struct ReadObjectData {
    pub metadata: entity::object::Model,
    pub file: Option<File>,
}

#[derive(Debug)]
pub struct WriteObjectData {
    pub binary: BodyDataStream,
    pub path: String,
    pub filename: Option<String>,
    pub encoded_filename: Option<String>,
    pub content_size: i64,
    pub mime_type: String,
}

// Multipart upload state management
pub type MultipartUploadState = Arc<Mutex<HashMap<String, MultipartUploadItem>>>;
pub static MULTIPART_UPLOAD_STATE: LazyLock<MultipartUploadState> = LazyLock::new(|| {
    Arc::new(Mutex::new(HashMap::new()))
});

#[derive(Clone)]
pub struct MultipartUploadItem {
    pub path: String,
    pub filename: Option<String>,
    pub encoded_filename: Option<String>,
    pub mime_type: String,
    pub last_upload_at: DateTime<Utc>,
}

pub async fn initialize() {
    let base_path = Path::new(&config::CONFIG.bucket.path);
    if !base_path.exists() {
        let result = fs::create_dir_all(base_path);
        if result.is_err() {
            tracing::error!("Failed to create bucket path: {}", result.unwrap_err());
            process::exit(1);
        } else {
            tracing::info!("Bucket dir created successfully: {}", base_path.display());
        }
    }

    let temp_path = base_path.join(".multipart");
    if temp_path.exists() {
        if let Err(e) = fs::remove_dir_all(&temp_path) {
            tracing::error!("Failed to remove expired multipart uploads: {}", e);
        }
    }
}

pub async fn get_object(path: String, with_file: bool) -> Result<ReadObjectData, Error> {
    let metadata = metadata::get_metadata_by_path(&path).await;
    if metadata.is_none() {
        return Err(anyhow::anyhow!("Object metadata not found for path: {}", path));
    }

    let object_data = metadata.unwrap();
    let internal_filename = object_data.internal_filename.clone();

    Ok(ReadObjectData {
        metadata: object_data,
        file: if with_file { Some(file::read_object(internal_filename).await?) } else { None },
    })
}

pub async fn put_object(data: WriteObjectData) -> Result<(), Error> {
    let internal_path = blake3::hash(data.path.as_bytes()).to_hex().to_string();
    let metadata = entity::object::ActiveModel {
        internal_filename: Set(internal_path.clone()),
        path: Set(data.path),
        filename: Set(data.filename),
        encoded_filename: Set(data.encoded_filename),
        content_size: Set(data.content_size),
        mime_type: Set(data.mime_type),
        ..Default::default()
    };

    metadata::create_metadata(metadata).await?;
    file::write_object(internal_path.clone(), data.binary, false).await?;

    Ok(())
}

pub fn create_multipart_upload(path: String, filename: Option<String>, encoded_filename: Option<String>, mime_type: String) -> String {
    let upload_id = Uuid::new_v4().to_string();
    let item = MultipartUploadItem {
        path,
        filename,
        encoded_filename,
        mime_type,
        last_upload_at: Utc::now(),
    };

    MULTIPART_UPLOAD_STATE.lock().unwrap().insert(upload_id.clone(), item);
    tracing::debug!("Multipart upload created with ID: {}", upload_id);

    tokio::spawn(internal_cleanup());
    upload_id
}

pub async fn upload_part(upload_id: String, number: u16, binary: BodyDataStream) -> Result<(), Error> {
    {
        let mut upload_item = MULTIPART_UPLOAD_STATE.lock().unwrap();
        let item = upload_item.get_mut(&upload_id);
        if item.is_none() {
            return Err(anyhow::anyhow!("Invalid or expired uploadId: {}", upload_id));
        }
        item.unwrap().last_upload_at = Utc::now();
    }

    file::write_object(format!("{upload_id}/{number}.part"), binary, true).await?;
    Ok(())
}

pub async fn complete_multipart_upload(upload_id: String) -> Result<(), Error> {
    let upload_item = {
        let mut state = MULTIPART_UPLOAD_STATE.lock().unwrap();
        state.remove(&upload_id)
    };

    if upload_item.is_none() {
        return Err(anyhow::anyhow!("Invalid or expired uploadId: {}", upload_id));
    }

    let item = upload_item.unwrap();
    let internal_filename = blake3::hash(item.path.as_bytes()).to_hex().to_string();
    let file_size = file::merge_partial_uploads(&upload_id, &internal_filename.clone()).await?;
    let metadata = entity::object::ActiveModel {
        internal_filename: Set(internal_filename),
        path: Set(item.path),
        filename: Set(item.filename),
        encoded_filename: Set(item.encoded_filename),
        content_size: Set(file_size as i64),
        mime_type: Set(item.mime_type),
        ..Default::default()
    };

    metadata::create_metadata(metadata).await?;
    file::delete_object(upload_id.clone(), true).await?;

    tracing::debug!("Multipart upload completed for ID: {}", upload_id);
    Ok(())
}

pub async fn abort_multipart_upload(upload_id: String) -> Result<(), Error> {
    {
        let mut state = MULTIPART_UPLOAD_STATE.lock().unwrap();
        state.remove(&upload_id);
    }

    if let Err(e) = file::delete_object(upload_id.clone(), true).await {
        tracing::error!("Failed to remove multipart upload directory: {}", e);
        return Err(e);
    }

    tracing::debug!("Multipart upload aborted for ID: {}", upload_id);
    Ok(())
}

pub async fn delete_object(path: String) -> Result<(), Error> {
    let metadata = metadata::get_metadata_by_path(&path).await;
    if metadata.is_none() {
        return Err(anyhow::anyhow!("Object metadata not found for path: {}", path));
    }
    let metadata = metadata.unwrap();

    file::delete_object(metadata.internal_filename.clone(), false).await?;
    metadata::delete_metadata(metadata).await?;

    tracing::debug!("Object deleted successfully at path: {}", path);
    Ok(())
}

static IS_CLEANUP_REGISTERED: AtomicBool = AtomicBool::new(false);

fn internal_cleanup() -> Pin<Box<dyn Future<Output=Result<(), ()>> + Send>> {
    Box::pin(async move {
        if IS_CLEANUP_REGISTERED.load(Ordering::SeqCst) {
            tracing::debug!("Cleanup already registered, skipping...");
            return Ok(());
        }

        let most_recent_item = {
            let state = MULTIPART_UPLOAD_STATE.lock().unwrap();
            let most_recent_item = state.values().min_by_key(|item| item.last_upload_at);

            if most_recent_item.is_none() {
                tracing::debug!("No cleanup needed, skipping...");
                return Ok(());
            }

            most_recent_item.unwrap().clone()
        };

        let exec_sec = config::CONFIG.bucket.request_expiration_seconds - (Utc::now() - most_recent_item.last_upload_at).num_seconds();
        if exec_sec > 0 {
            tracing::debug!("Scheduling cleanup in {} seconds...", exec_sec);
            IS_CLEANUP_REGISTERED.store(true, Ordering::SeqCst);
            time::sleep(Duration::from_secs((exec_sec as u64) + 1)).await;
        }

        tracing::debug!("Starting cleanup of expired multipart uploads...");
        let expired_uploads = {
            let state = MULTIPART_UPLOAD_STATE.lock().unwrap();
            let now = Utc::now();
            let diff = TimeDelta::seconds(config::CONFIG.bucket.request_expiration_seconds);
            state.iter()
                .filter_map(|(id, item)| {
                    if now - item.last_upload_at > diff {
                        Some(id.clone())
                    } else {
                        None
                    }
                })
                .collect::<Vec<String>>()
        };

        for upload_id in expired_uploads {
            tracing::debug!("Removing expired multipart upload: {}", upload_id);
            if let Err(e) = file::delete_object(upload_id.clone(), true).await {
                tracing::error!("Failed to remove expired multipart upload {}: {}", upload_id, e);
            }
            MULTIPART_UPLOAD_STATE.lock().unwrap().remove(&upload_id);
        }

        IS_CLEANUP_REGISTERED.store(false, Ordering::SeqCst);
        tracing::debug!("Cleanup completed.");

        tokio::spawn(internal_cleanup());
        Ok(())
    })
}
