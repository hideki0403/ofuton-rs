use once_cell::sync::Lazy;
use serde::Deserialize;
use config::{Config, File};
use crate::resource;

#[derive(Debug, Deserialize, Clone)]
pub struct CFGServer {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CFGDatabase {
    pub connection: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CFGBucket {
    pub path: String,
    pub max_upload_size: u64,
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

// https://blog.logrocket.com/configuration-management-in-rust-web-services/
impl AppConfig {
    pub fn new() -> Result<Self, config::ConfigError> {
        Config::builder()
            .add_source(File::with_name("./config.toml"))
            .build()
            .expect("Failed to load configuration")
    }
}

pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    Config::builder()
        .add_source(File::with_name("./config.toml"))
        .build()
        .expect("Failed to load configuration")
});