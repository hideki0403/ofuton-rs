use std::path::{Path, PathBuf};
use anyhow::Error;
use axum::body::BodyDataStream;
use tokio::{fs::{self, File}, io::{AsyncWriteExt, BufWriter}};
use tokio_stream::StreamExt;

use crate::config;

pub async fn read_object(internal_filename: String) -> Result<File, Error> {
    let path = resolve_path(internal_filename);
    if !path.exists() {
        return Err(anyhow::anyhow!("File does not exist"));
    }

    let file = File::open(&path).await;
    if file.is_err() {
        return Err(anyhow::anyhow!("Failed to open file: {}", file.as_ref().err().unwrap()));
    }

    return Ok(file.unwrap());
}

pub async fn write_object(internal_filename: String, mut stream: BodyDataStream, create_dir: bool) -> Result<(), Error> {
    let path = resolve_path(internal_filename);
    if path.exists() {
        return Err(anyhow::anyhow!("File already exists at path: {}", path.display()));
    }

    if create_dir && let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).await?;
        }
    }

    let mut writer = BufWriter::new(File::create(&path).await?);
    while let Some(chunk) = stream.next().await {
        writer.write_all(&chunk?).await?;
    }

    writer.flush().await?;
    return Ok(());
}

fn resolve_path(internal_filename: String) -> PathBuf {
    return Path::new(&config::CONFIG.bucket.path).join(internal_filename);
}