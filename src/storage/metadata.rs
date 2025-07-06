use anyhow::Error;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use crate::entity;
use crate::database;

pub async fn get_metadata_by_path(path: &str) -> Option<entity::object::Model> {
    let object_data = entity::object::Entity::find()
        .filter(entity::object::Column::Path.eq(path))
        .one(database::get_db())
        .await;


    if object_data.is_err() {
        tracing::error!("Failed to fetch object metadata for path '{}': {}", path, object_data.as_ref().err().unwrap());
        return None;
    }

    return object_data.unwrap();
}

pub async fn create_metadata(model: entity::object::ActiveModel) -> Result<(), Error> {
    let insert_result = entity::object::Entity::insert(model)
        .exec(database::get_db())
        .await;

    if insert_result.is_err() {
        tracing::error!("Failed to create object metadata: {}", insert_result.as_ref().err().unwrap());
        return Err(insert_result.unwrap_err().into());
    }

    return Ok(());
}