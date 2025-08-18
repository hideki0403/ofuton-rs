use crate::config;
use migration::{Migrator, MigratorTrait};
use once_cell::sync::OnceCell;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use tracing::log::LevelFilter;

static DB: OnceCell<DatabaseConnection> = OnceCell::new();

pub async fn initialize() -> Result<(), sea_orm::DbErr> {
    let conf = config::CONFIG.clone();
    let path = match conf.database.provider.as_str() {
        "sqlite" => format!("sqlite://{}?mode=rwc", conf.database.sqlite.path),
        "sqlite_memory" => "sqlite::memory:".to_string(),
        "postgres" => format!(
            "postgres://{}:{}@{}:{}/{}",
            conf.database.postgres.user,
            conf.database.postgres.password,
            conf.database.postgres.host,
            conf.database.postgres.port,
            conf.database.postgres.database
        ),
        _ => panic!("Unsupported database provider"),
    };

    let mut options = ConnectOptions::new(path);
    options.sqlx_logging_level(LevelFilter::Debug);

    let connection = Database::connect(options).await;
    if let Err(err) = connection {
        tracing::error!("Failed to connect to the database: {}", err);
        return Err(err);
    }

    let db = connection.unwrap();
    let migration = Migrator::up(&db, None).await;
    if let Err(err) = migration {
        tracing::error!("Failed to apply migrations: {}", err);
        return Err(err);
    }

    DB.set(db).expect("Database already initialized");
    Ok(())
}

pub fn get_db() -> &'static DatabaseConnection {
    DB.get().expect("Database has not been initialized")
}
