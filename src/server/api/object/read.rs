use axum::{extract::Path, http::{HeaderMap, Method, StatusCode}, response::IntoResponse};
use axum_range::{Ranged, KnownSize};
use axum_extra::{headers::Range, TypedHeader};
use crate::server::{AppError, AppResult};
use crate::storage;

pub async fn read_handler(method: Method, Path(object_path): Path<String>, range: Option<TypedHeader<Range>>) -> AppResult<impl IntoResponse> {
    if object_path.is_empty() {
        return Ok(StatusCode::BAD_REQUEST.into_response());
    }

    let is_head_request = method == Method::HEAD;
    let object_data = storage::get_object(object_path, !is_head_request).await;
    if object_data.is_err() {
        return Err(AppError(object_data.unwrap_err()));
    }
    let object_data = object_data.unwrap();

    // set headers
    let mut headers = HeaderMap::new();
    headers.insert("Cache-Control", "max-age=31536000, immutable".parse().unwrap());
    headers.insert("Content-Type", object_data.metadata.mime_type.parse().unwrap());
    headers.insert("Content-Length", object_data.metadata.content_size.to_string().parse().unwrap());
    headers.insert("Content-Disposition", format!("inline; filename=\"{}\"", object_data.metadata.filename).parse().unwrap());
    headers.insert("ETag", format!("\"{}\"", object_data.metadata.internal_filename).parse().unwrap());

    if is_head_request {
        return Ok((StatusCode::OK, headers).into_response());
    }

    let range = range.map(|TypedHeader(range)| range);
    return Ok(Ranged::new(range, KnownSize::file(object_data.file.unwrap()).await.unwrap()).into_response());
}
