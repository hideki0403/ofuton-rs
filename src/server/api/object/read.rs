use crate::{
    server::{AppResult, utils::build_content_disposition_filename},
    storage,
};
use axum::{
    body::Body,
    extract::Request,
    http::{HeaderMap, Method, StatusCode},
    response::IntoResponse,
};
use axum_extra::{TypedHeader, headers::Range};
use axum_range::{KnownSize, Ranged};

pub async fn read_handler(method: Method, range: Option<TypedHeader<Range>>, request: Request<Body>) -> AppResult<impl IntoResponse> {
    let object_path = request.uri().path().to_string();
    if object_path.is_empty() {
        return Ok(StatusCode::BAD_REQUEST.into_response());
    }

    let is_head_request = method == Method::HEAD;
    let object_data = storage::get_object(object_path, !is_head_request).await;
    if object_data.is_err() {
        return Ok((StatusCode::NOT_FOUND, "Object not found").into_response());
    }
    let object_data = object_data.unwrap();

    // set headers
    let mut headers = HeaderMap::new();
    headers.insert("Cache-Control", "max-age=31536000, immutable".parse().unwrap());
    headers.insert("Content-Type", object_data.metadata.mime_type.parse().unwrap());
    headers.insert("ETag", format!("\"{}\"", object_data.metadata.internal_filename).parse().unwrap());
    headers.insert("Accept-Ranges", "bytes".parse().unwrap());

    let mut content_disposition = vec!["inline".to_string()];
    content_disposition.extend(build_content_disposition_filename(
        object_data.metadata.filename,
        object_data.metadata.encoded_filename,
    ));
    headers.insert("Content-Disposition", content_disposition.join("; ").parse().unwrap());

    if is_head_request {
        headers.insert("Content-Length", object_data.metadata.content_size.to_string().parse().unwrap());
        return Ok((StatusCode::OK, headers).into_response());
    }

    let range = range.map(|TypedHeader(range)| range);
    let mut response = Ranged::new(range, KnownSize::file(object_data.file.unwrap()).await.unwrap()).into_response();
    response.headers_mut().extend(headers);

    Ok(response)
}
