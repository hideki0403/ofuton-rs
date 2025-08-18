use crate::database;
use anyhow::Error;
use sea_orm::{ColumnTrait, EntityTrait, ModelTrait, QueryFilter};

pub async fn get_metadata_by_path(path: &str) -> Option<entity::object::Model> {
    let object_data = entity::object::Entity::find()
        .filter(entity::object::Column::Path.eq(path))
        .one(database::get_db())
        .await;

    if let Err(e) = object_data {
        tracing::error!("Failed to fetch object metadata for path '{}': {}", path, e);
        return None;
    }

    object_data.unwrap()
}

pub async fn create_metadata(model: entity::object::ActiveModel) -> Result<(), Error> {
    let insert_result = entity::object::Entity::insert(model).exec(database::get_db()).await;

    if let Err(e) = insert_result {
        tracing::error!("Failed to create object metadata: {}", e);
        return Err(e.into());
    }

    Ok(())
}

#[allow(dead_code)] // TODO: Remove
pub async fn create_metadata_many(models: Vec<entity::object::ActiveModel>) -> Result<(), Error> {
    let insert_result = entity::object::Entity::insert_many(models)
        .on_empty_do_nothing()
        .exec(database::get_db())
        .await;

    if let Err(e) = insert_result {
        tracing::error!("Failed to create multiple object metadata: {}", e);
        return Err(e.into());
    }

    Ok(())
}

pub async fn delete_metadata(model: entity::object::Model) -> Result<(), Error> {
    let delete_result = model.delete(database::get_db()).await;

    if let Err(e) = delete_result {
        tracing::error!("Failed to delete object metadata: {}", e);
        return Err(e.into());
    }

    Ok(())
}
