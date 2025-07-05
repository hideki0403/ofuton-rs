use tokio;
use sentry;

mod database;
mod entity;
mod config;
mod resource;

fn main() {
    // Logging setup
    let filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        let level = if cfg!(debug_assertions) {
            "debug"
        } else {
            "info"
        };
        tracing_subscriber::EnvFilter::new(level)
    });

    tracing_subscriber::fmt().with_env_filter(filter).init();

    // Sentry setup
    let conf = config::CONFIG.clone();
    if let Ok(dsn) = sentry::IntoDsn::into_dsn(conf.sentry.dsn) {
        tracing::info!("Sentry logging is enabled");
        let _guard = sentry::init(sentry::ClientOptions {
            dsn: dsn,
            release: sentry::release_name!(),
            ..Default::default()
        });
    }

    // Start the Tokio runtime
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime")
        .block_on(async {
            run().await;
        });
}

async fn run() {
    database::initialize().await.expect("Failed to initialize the database");
}
