use entity;
use dialoguer::Confirm;
use sea_orm::ActiveValue::Set;
use sea_orm::ColumnTrait;
use sea_orm::EntityTrait;
use sea_orm::QueryFilter;
use sea_orm::TransactionTrait;
use serde::Deserialize;
use url::Url;
use crate::database;
use crate::cli::utils;

#[derive(Debug, Deserialize)]
struct DriveFile {
    name: String,
    mime_type: String,
    url: String,
}

pub async fn execute(metadata_path: String) {
    tracing::info!("Loading metadata from {}", metadata_path);

    let mut reader_builder = csv::ReaderBuilder::new();
    reader_builder.delimiter(b'\t');
    reader_builder.has_headers(false);

    let reader = reader_builder.from_path(metadata_path);
    if reader.is_err() {
        tracing::error!("Failed to read metadata file: {}", reader.unwrap_err());
        return;
    }

    let mut records = reader.unwrap();
    let mut drive_files = Vec::new();

    for result in records.deserialize() {
        if result.is_err() {
            tracing::error!("Failed to deserialize record: {}", result.unwrap_err());
            continue;
        }

        let record: DriveFile = result.unwrap();
        drive_files.push(record);
    }

    if drive_files.is_empty() {
        tracing::warn!("No valid entries found in the metadata file.");
        return;
    }

    tracing::info!("Found {} entries. Ready to import.", drive_files.len());
    let confirmation = Confirm::new()
        .with_prompt("Continue?")
        .interact()
        .unwrap();

    if !confirmation {
        tracing::info!("Import cancelled.");
        return;
    }

    let pb = utils::create_progress_bar(drive_files.len() as u64);
    let db = database::get_db();
    let chunk_size = 100;

    for chunk in drive_files.chunks(chunk_size) {
        let txn = match db.begin().await {
            Ok(txn) => txn,
            Err(e) => {
                tracing::error!("Failed to begin transaction: {}", e);
                return;
            }
        };

        for record in chunk {
            let filepath = match Url::parse(&record.url) {
                Ok(url) => url.path().to_string(),
                Err(e) => {
                    tracing::error!("Invalid URL {}: {}", record.url, e);
                    continue;
                }
            };

            let (normalized_filename, encoded_filename) = if utils::FILENAME_NORMALIZE_REGEX.is_match(&record.name) {
                let normalized_filename = utils::FILENAME_NORMALIZE_REGEX.replace_all(&record.name, "_").to_string();
                let encoded_filename = urlencoding::encode(&record.name).to_string();
                (normalized_filename, Some(encoded_filename))
            } else {
                (record.name.clone(), None)
            };

            let result = entity::object::Entity::update_many()
                .set(entity::object::ActiveModel {
                    filename: Set(Some(normalized_filename)),
                    encoded_filename: Set(encoded_filename),
                    mime_type: Set(record.mime_type.clone()),
                    ..Default::default()
                })
                .filter(entity::object::Column::Filename.is_null())
                .filter(entity::object::Column::Path.eq(&filepath))
                .exec(&txn)
                .await;

            if let Err(e) = result {
                tracing::error!("Failed to update record for url {}: {}", record.url, e);
                if let Err(rollback_err) = txn.rollback().await {
                    tracing::error!("Failed to rollback transaction: {}", rollback_err);
                }
                return;
            }

            pb.inc(1);
        }

        if let Err(e) = txn.commit().await {
            tracing::error!("Failed to commit transaction: {}", e);
            return;
        }
    }

    tracing::info!("Successfully processed {} entries.", drive_files.len());
}