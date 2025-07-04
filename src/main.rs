
use tokio;

mod database;
mod entity;
mod config;
mod resource;

#[tokio::main]
async fn main() {
    database::initialize().await.expect("Failed to initialize the database");
}
