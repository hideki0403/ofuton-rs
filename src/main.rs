
use tokio;
use sentry;

mod database;
mod entity;
mod config;
mod resource;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let conf = config::CONFIG.clone();
    if let Ok(dsn) = sentry::IntoDsn::into_dsn(conf.sentry.dsn) {
        tracing::info!("Sentry logging is enabled");
        let _guard = sentry::init(sentry::ClientOptions {
            dsn: dsn,
            release: sentry::release_name!(),
            ..Default::default()
        });
    }

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
