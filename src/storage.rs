use anyhow::Error;
use tokio::fs::File;
use crate::entity;

mod file;
mod metadata;

#[derive(Debug)]
pub struct ObjectData {
    pub metadata: entity::object::Model,
    pub file: Option<File>,
}

pub async fn get_object(path: String, with_file: bool) -> Result<ObjectData, Error> {
    let metadata = metadata::get_metadata_by_path(&path).await;
    if metadata.is_none() {
        return Err(anyhow::anyhow!("Object metadata not found for path: {}", path));
    }

    let object_data = metadata.unwrap();
    let internal_filename = object_data.internal_filename.clone();

    Ok(ObjectData {
        metadata: object_data,
        file: if with_file { Some(file::read_object(internal_filename).await?) } else { None },
    })
}

pub async fn write_object(path: &str, file: File) -> Result<(), Error> {
    // TODO
}