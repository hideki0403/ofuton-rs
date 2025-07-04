use std::{fs, path::Path};
use once_cell::sync::Lazy;
use serde::Deserialize;
use config::{Config, File, FileFormat};
use crate::resource;

#[derive(Debug, Deserialize, Clone)]
pub struct CFGServer {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CFGDatabase {
    pub provider: String,
    pub sqlite: CFGDatabaseSQLite,
    pub postgres: CFGDatabasePostgres,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CFGDatabaseSQLite {
    pub path: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CFGDatabasePostgres {
    pub user: String,
    pub password: String,
    pub host: String,
    pub port: u16,
    pub database: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CFGBucket {
    pub path: String,
    pub max_upload_size_mb: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CFGAccount {
    pub access_key: String,
    pub secret_key: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub server: CFGServer,
    pub database: CFGDatabase,
    pub bucket: CFGBucket,
    pub account: CFGAccount,
}

impl AppConfig {
    pub fn new() -> Result<Self, config::ConfigError> {
        if !Path::new("./config.toml").exists() {
            let default_config = resource::DEFAULT_CONFIG_TOML;
            fs::write("./config.toml", default_config).expect("Failed to create default config file");
        }

        let config = Config::builder()
            .add_source(File::from_str(str::from_utf8(resource::DEFAULT_CONFIG_TOML).unwrap(), FileFormat::Toml))
            .add_source(File::with_name("./config.toml"))
            .build()
            .expect("Failed to load configuration");

        config.try_deserialize::<AppConfig>()
    }
}

pub static CONFIG: Lazy<AppConfig> = Lazy::new(|| {
    AppConfig::new().expect("Failed to initialize application configuration")
});