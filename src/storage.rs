use std::fs;
use std::path::Path;
use std::process;
use anyhow::Error;
use axum::body::BodyDataStream;
use sea_orm::ActiveValue::Set;
use tokio::fs::File;
use crate::entity;
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
    pub filename: String,
    pub content_size: i64,
    pub mime_type: String,
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

pub async fn write_object(data: WriteObjectData) -> Result<(), Error> {
    let internal_path = blake3::hash(data.path.as_bytes()).to_hex().to_string();
    let metadata = entity::object::ActiveModel {
        internal_filename: Set(internal_path.clone()),
        path: Set(data.path),
        filename: Set(data.filename),
        content_size: Set(data.content_size),
        mime_type: Set(data.mime_type),
        ..Default::default()
    };

    metadata::create_metadata(metadata).await?;
    file::write_object(internal_path.clone(), data.binary).await?;

    return Ok(());
}