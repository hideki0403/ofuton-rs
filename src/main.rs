
use tokio;
use sea_orm::Database;
use migration::{Migrator, MigratorTrait};

mod config;
mod resource;

#[tokio::main]
async fn main() {
    let db = Database::connect(config).await?;
    Migrator::up(db, None).await?;
}
