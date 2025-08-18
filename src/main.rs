use tracing_subscriber::filter::LevelFilter;

mod cli;
mod config;
mod database;
mod resource;
mod server;
mod storage;

fn main() {
    let conf = config::CONFIG.clone();

    // Logging setup
    let filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        let level = conf
            .debug
            .as_ref()
            .and_then(|d| d.log_level.as_ref())
            .and_then(|s| s.parse::<LevelFilter>().ok())
            .unwrap_or({ if cfg!(debug_assertions) { LevelFilter::DEBUG } else { LevelFilter::INFO } });

        tracing_subscriber::EnvFilter::new(level.to_string())
    });

    tracing_subscriber::fmt().with_env_filter(filter).init();

    // Sentry setup
    if !conf.sentry.dsn.is_empty() &&
        let Ok(dsn) = sentry::IntoDsn::into_dsn(conf.sentry.dsn)
    {
        tracing::info!("Sentry logging is enabled");
        let _guard = sentry::init(sentry::ClientOptions {
            dsn,
            release: sentry::release_name!(),
            ..Default::default()
        });
    }

    // Handle argments
    let command = cli::handle();

    // Start the Tokio runtime
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime")
        .block_on(async {
            run(command).await;
        });
}

async fn run(command: Option<cli::MigrationCommand>) {
    database::initialize().await.expect("Failed to initialize the database");
    storage::initialize().await;

    if command.is_none() {
        server::listen().await;
    } else {
        cli::execute(command.unwrap()).await;
    }
}
