use axum::http::{HeaderMap, HeaderValue};
use once_cell::sync::Lazy;
use regex::Regex;

pub fn get_header(header: &HeaderMap<HeaderValue>, header_name: &str, fallback: Option<String>) -> String {
    header.get(header_name)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or(fallback.unwrap_or_default())
}

static CONTENT_DISPOSITION_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"filename\*=(?i)utf(?-i)-?8''(?<filename>.*?)(?:;|$)").expect("Failed to compile regex")
});

#[derive(Debug, Default)]
pub struct ContentDisposition {
    pub filename: Option<String>,
    pub encoded_filename: Option<String>,
}

pub fn parse_content_disposition(content_disposition: &str) -> ContentDisposition {
    if content_disposition.is_empty() {
        return ContentDisposition::default();
    }

    let mut result = ContentDisposition::default();

    if let Some(caps) = CONTENT_DISPOSITION_REGEX.captures(content_disposition) && let Some(filename) = caps.name("filename") {
        let raw_filename = filename.as_str();
        if let Ok(decoded) = urlencoding::decode(raw_filename) {
            let decoded = decoded.into_owned();
            if decoded != raw_filename {
                result.encoded_filename = Some(raw_filename.to_string());
            } else {
                // もし送られてきたファイル名がURLエンコードされていなければURLエンコードしておく
                result.encoded_filename = Some(urlencoding::encode(&decoded).to_string());
            }
        }
    }

    let filename = content_disposition.split(';')
        .find(|part| part.trim().starts_with("filename="))
        .and_then(|part| part.split('=').nth(1))
        .map(|filename| filename.trim_matches('"').to_string());

    result.filename = filename;
    return result;
}

pub fn build_content_disposition_filename(filename: String, encoded_filename: Option<String>) -> String {
    if filename.is_empty() {
        return "".to_string();
    }

    if let Some(encoded) = encoded_filename {
        return format!("filename=\"{}\"; filename*=utf-8''{}", filename, encoded);
    } else {
        return format!("filename=\"{}\"", filename);
    }
}
