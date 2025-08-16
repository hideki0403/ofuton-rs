use std::path::{Path, PathBuf, MAIN_SEPARATOR};
use dialoguer::Confirm;
use indicatif::ProgressBar;
use sea_orm::ActiveValue::Set;
use sea_orm::EntityTrait;
use tokio::fs;
use async_recursion::async_recursion;
use mime_guess;
use crate::cli::utils;
use crate::database;
use crate::entity;
use crate::config;

#[derive(Debug)]
pub struct MigrateObject {
    pub path: PathBuf,
    pub internal_filename: String,
    pub model: entity::object::ActiveModel,
}

pub async fn execute(old_dir: String) {
    tracing::info!("Calcurating files to migrate from old directory: {}", old_dir);
    let total_files = match count_files_recursive(&old_dir).await {
        Ok(count) => count,
        Err(e) => {
            tracing::error!("Failed to count files: {}", e);
            return;
        }
    };

    if total_files == 0 {
        tracing::info!("No files to migrate.");
        return;
    }
    tracing::info!("Found {} files to migrate.", total_files);

    let confirmation = Confirm::new()
        .with_prompt("Continue?")
        .interact()
        .unwrap();

    if !confirmation {
        tracing::info!("Migration cancelled.");
        return;
    }

    let pb = utils::create_progress_bar(total_files);
    let mut items: Vec<MigrateObject> = Vec::new();
    if let Err(e) = migrate_objects_recursive(&old_dir, &old_dir, &mut items, &pb).await {
        tracing::error!("Failed to migrate objects from old directory: {}", e);
        return;
    }

    pb.finish();
    tracing::info!("Migration completed successfully. If necessary, run the `import` command. (The `import` command imports accurate file information from Misskey)");
}

#[async_recursion]
async fn count_files_recursive(current_dir: &str) -> Result<u64, anyhow::Error> {
    let mut count = 0;
    let mut entries = fs::read_dir(current_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let filetype = entry.file_type().await?;
        if filetype.is_dir() {
            count += count_files_recursive(&entry.path().to_string_lossy()).await?;
        } else if filetype.is_file() {
            count += 1;
        }
    }
    Ok(count)
}

#[async_recursion]
async fn migrate_objects_recursive(base_dir: &str, current_dir: &str, items: &mut Vec<MigrateObject>, pb: &ProgressBar) -> Result<(), anyhow::Error> {
    let mut entries = fs::read_dir(current_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let filetype = entry.file_type().await?;
        if filetype.is_dir() {
            migrate_objects_recursive(&base_dir, &entry.path().to_string_lossy(), items, pb).await?;
            continue;
        } else if !filetype.is_file() {
            continue;
        }

        let path = entry.path();
        let relative_path = path.strip_prefix(base_dir)?;
        let relative_path_str = format!("/{}", relative_path.to_string_lossy().to_string().replace(MAIN_SEPARATOR, "/"));

        let mime = mime_guess::from_path(&path).first_or_octet_stream().to_string();
        // let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or_default().to_string();
        // let normalized_filename = utils::FILENAME_NORMALIZE_REGEX.replace_all(&filename, "_").to_string();
        let internal_filename = blake3::hash(relative_path_str.as_bytes()).to_hex().to_string();

        let object = entity::object::ActiveModel {
            path: Set(relative_path_str),
            // filename: Set(normalized_filename),
            content_size: Set(entry.metadata().await?.len() as i64),
            mime_type: Set(mime),
            internal_filename: Set(internal_filename.clone()),
            ..Default::default()
        };

        // if utils::FILENAME_NORMALIZE_REGEX.is_match(&filename) {
        //     let encoded_filename = urlencoding::encode(&filename).to_string();
        //     object.encoded_filename = Set(Some(encoded_filename));
        // }

        items.push(MigrateObject {
            path: path.clone(),
            internal_filename: internal_filename,
            model: object,
        });

        if items.len() >= 50 {
            migrate_objects(items, pb).await?;
        }
    }

    migrate_objects(items, pb).await?;
    Ok(())
}

async fn migrate_objects(items: &mut Vec<MigrateObject>, pb: &ProgressBar) -> Result<(), anyhow::Error> {
    if items.is_empty() {
        return Ok(());
    }

    let models = items.iter().map(|item| item.model.clone()).collect::<Vec<_>>();
    entity::object::Entity::insert_many(models).exec(database::get_db()).await?;

    for item in &mut *items {
        let dist = Path::new(&config::CONFIG.bucket.path).join(&item.internal_filename);
        fs::rename(&item.path, dist).await?;
    }

    pb.inc(items.len() as u64);
    items.clear();
    Ok(())
}
