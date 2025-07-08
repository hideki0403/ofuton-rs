use std::{sync::Arc, time::Duration};
use axum::{extract::DefaultBodyLimit, http::StatusCode, response::{IntoResponse, Response}, routing, Router};
use tokio::net::TcpListener;
use tower_http::{request_id::{MakeRequestUuid, SetRequestIdLayer}, trace::TraceLayer};
use tracing::Span;
use uuid::Uuid;
use crate::config;

mod middleware;
mod api;
mod utils;

// Error handling
pub struct AppError(anyhow::Error);

impl<E> From<E> for AppError where E: Into<anyhow::Error>, {
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let request_id = Uuid::new_v4();
        tracing::error!(request_id = %request_id, "{}", self.0);
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Internal Server Error (RequestID: {})", request_id)).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;

// Server setup
pub async fn listen() {
    let conf = config::CONFIG.clone();
    let verify_signatures = middleware::signature::SignatureVerificationState {
        key: Arc::new(conf.account.access_key.clone()),
        secret: Arc::new(conf.account.secret_key.clone()),
    };

    let write_routes = Router::new()
        .route("/{*object}", routing::post(api::object::write::write_handler).put(api::object::write::write_handler).delete(api::object::write::write_handler))
        .layer(DefaultBodyLimit::max((conf.bucket.max_upload_size_mb * 1024 * 1024).try_into().unwrap()))
        .layer(axum::middleware::from_fn_with_state(verify_signatures, middleware::signature::signature_verification));

    let app = Router::new()
        .route("/", api::r#static::index())
        .route("/robots.txt", api::r#static::robots_txt())
        .route("/{*object}", routing::get(api::object::read::read_handler).head(api::object::read::read_handler))
        .merge(write_routes)
        .layer(axum::middleware::from_fn(middleware::logger::request_logger))
        .layer(
            TraceLayer::new_for_http()
                .on_response(|response: &Response, latency: Duration, _: &Span| {
                    if let Some(request_logger) = response.extensions().get::<middleware::logger::RequestLogger>() {
                        tracing::info!(
                            parent: None,
                            "{} {} {} ({:.1}ms)",
                            request_logger.method,
                            response.status(),
                            request_logger.uri,
                            latency.as_secs_f64() * 1000.0
                        );
                    }
                })
        )
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid));

    let addr = format!("{}:{}", conf.server.host, conf.server.port);
    let listener = TcpListener::bind(&addr).await;
    if let Err(err) = listener {
        tracing::error!("Failed to bind to {}: {}", addr, err);
        return;
    }

    tracing::info!("Server listening on http://{}", addr);

    let server = axum::serve(listener.unwrap(), app).await;
    if let Err(err) = server {
        tracing::error!("Server error: {}", err);
    }
}