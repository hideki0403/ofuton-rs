use axum::routing::{MethodRouter, get};

pub fn index() -> MethodRouter {
    get(|| async {
        let version = env!("CARGO_PKG_VERSION");
        format!("ofuton v{version} - https://github.com/hideki0403/ofuton-rs")
    })
}

pub fn robots_txt() -> MethodRouter {
    get(|| async { "User-agent: *\nDisallow: /" })
}
