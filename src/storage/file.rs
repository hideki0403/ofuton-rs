use std::path::Path;
use anyhow::Error;
use tokio::fs::File;

use crate::config;

pub async fn read_object(internal_filename: String) -> Result<File, Error> {
    let path = Path::new(&config::CONFIG.bucket.path).join(internal_filename);

    if !path.exists() {
        return Err(anyhow::anyhow!("File does not exist"));
    }

    let file = File::open(&path).await;
    if file.is_err() {
        return Err(anyhow::anyhow!("Failed to open file: {}", file.as_ref().err().unwrap()));
    }

    return Ok(file.unwrap());
}

pub async fn write_object() -> Result<(), Error> {
    // TODO
}
