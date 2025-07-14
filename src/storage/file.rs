use std::path::{Path, PathBuf};
use anyhow::Error;
use axum::body::BodyDataStream;
use tokio::{fs::{self, File, OpenOptions}, io::{self, AsyncWriteExt, BufWriter}};
use tokio_stream::StreamExt;

use crate::config;

pub async fn read_object(internal_filename: String) -> Result<File, Error> {
    let path = resolve_path(internal_filename, false);
    if !path.exists() {
        return Err(anyhow::anyhow!("File does not exist"));
    }

    let file = File::open(&path).await;
    if file.is_err() {
        return Err(anyhow::anyhow!("Failed to open file: {}", file.as_ref().err().unwrap()));
    }

    tracing::debug!("Object read successfully from path: {}", path.display());
    return Ok(file.unwrap());
}

pub async fn write_object(internal_filename: String, mut stream: BodyDataStream, is_multipart: bool) -> Result<(), Error> {
    let path = resolve_path(internal_filename, is_multipart);
    if path.exists() {
        return Err(anyhow::anyhow!("File already exists at path: {}", path.display()));
    }

    if is_multipart && let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).await?;
        }
    }

    let mut writer = BufWriter::new(File::create(&path).await?);
    while let Some(chunk) = stream.next().await {
        writer.write_all(&chunk?).await?;
    }

    writer.flush().await?;

    tracing::debug!("Object written successfully to path: {}", path.display());
    return Ok(());
}

pub async fn merge_partial_uploads(upload_id: &String, internal_filename: &String) -> Result<u64, Error> {
    let object_path = resolve_path(internal_filename.clone(), false);
    let multipart_path = resolve_path(upload_id.clone(), true);
    let temporary_output_path = multipart_path.join("object-merged.tmp");

    if !multipart_path.exists() {
        return Err(anyhow::anyhow!("Multipart upload path does not exist: {}", multipart_path.display()));
    }

    if object_path.exists() {
        return Err(anyhow::anyhow!("File already exists at path: {}", object_path.display()));
    }

    let mut multipart_files = fs::read_dir(&multipart_path).await?;
    let mut file_list = Vec::new();

    while let Some(entry) = multipart_files.next_entry().await? {
        if entry.file_type().await?.is_file() {
            file_list.push(entry.path());
        }
    };

    file_list.sort_by(|a, b| {
        let a_name = a.file_name().unwrap().to_str().unwrap().replace(".part", "");
        let b_name = b.file_name().unwrap().to_str().unwrap().replace(".part", "");
        let a_num = a_name.parse::<u64>().unwrap_or(0);
        let b_num = b_name.parse::<u64>().unwrap_or(0);
        a_num.cmp(&b_num)
    });

    let output_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&temporary_output_path)
        .await?;

    let mut writer = BufWriter::new(output_file);

    for file in file_list {
        let mut input_file = File::open(file).await?;
        io::copy(&mut input_file, &mut writer).await?;
    }

    writer.flush().await?;
    fs::rename(&temporary_output_path, &object_path).await?;

    tracing::debug!("Merged multipart uploads into object at path: {}", object_path.display());
    return Ok(fs::metadata(&object_path).await?.len());
}

pub async fn delete_object(internal_path: String, is_multipart: bool) -> Result<(), Error> {
    let path = resolve_path(internal_path, is_multipart);

    if is_multipart {
        fs::remove_dir_all(&path).await?;
    } else {
        if !path.exists() {
            return Err(anyhow::anyhow!("File does not exist at path: {}", path.display()));
        }

        fs::remove_file(&path).await?;
    }

    tracing::debug!("Deleted object at path: {}", path.display());
    return Ok(());
}

fn resolve_path(internal_path: String, is_multipart: bool) -> PathBuf {
    let base = Path::new(&config::CONFIG.bucket.path);
    if is_multipart {
        base.join(".multipart").join(internal_path)
    } else {
        base.join(internal_path)
    }
}
