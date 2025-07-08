use axum::{body::Body, extract::Request};

pub fn get_header(request: &Request<Body>, header_name: &str, fallback: Option<String>) -> String {
    request.headers().get(header_name)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or(fallback.unwrap_or_default())
}

pub fn parse_content_disposition(content_disposition: &str) -> Option<String> {
    // TODO: 日本語を含むファイル名が来たときの挙動を確認する
    println!("parse_content_disposition: {}", content_disposition);
    content_disposition.split(';')
        .find(|part| part.trim().starts_with("filename="))
        .and_then(|part| part.split('=').nth(1))
        .map(|filename| filename.trim_matches('"').to_string())
}