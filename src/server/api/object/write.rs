use axum::{body::Body, extract::{Path, Request}, http::{Method, Response, StatusCode}, response::IntoResponse};
use serde::Serialize;
use uuid::Uuid;
use crate::storage;
use crate::server::AppResult;
use crate::server::middleware::multipart::MultipartUploadState;
use crate::server::utils::{get_header, parse_content_disposition};

#[derive(PartialEq, Debug)]
enum OperationType {
    PutObject,
    CreateMultipartUpload,
    UploadPart,
    CompleteMultipartUpload,
    AbortMultipartUpload,
    DeleteObject,
    Unknown,
}

pub async fn write_handler(Path(object_path): Path<String>, request: Request<Body>) -> AppResult<impl IntoResponse> {
    if object_path.is_empty() {
        return Ok(StatusCode::BAD_REQUEST.into_response());
    }

    let (parts, body) = request.into_parts();
    let multipart_upload_state = parts.extensions.get::<MultipartUploadState>().unwrap();
    let is_multipart_operation = multipart_upload_state.upload_id.as_ref().is_some();

    let operation = match parts.method {
        Method::PUT => {
            if is_multipart_operation {
                OperationType::UploadPart
            } else {
                OperationType::PutObject
            }
        }
        Method::POST => {
            if is_multipart_operation {
                OperationType::CompleteMultipartUpload
            } else {
                OperationType::CreateMultipartUpload
            }
        }
        Method::DELETE => {
            if is_multipart_operation {
                OperationType::AbortMultipartUpload
            } else {
                OperationType::DeleteObject
            }
        },
        _ => OperationType::Unknown,
    };

    if operation == OperationType::Unknown {
        return Ok((StatusCode::BAD_REQUEST, "unknown operation").into_response());
    }

    tracing::debug!("Operation: {:?}", operation);

    let mime_type = get_header(&parts.headers, "Content-Type", Some("application/octet-stream".to_string()));
    let content_size = get_header(&parts.headers, "Content-Length", None).parse::<i64>().unwrap_or(0);
    let filename = parse_content_disposition(get_header(&parts.headers, "Content-Disposition", None).as_str()).unwrap_or("unknown".to_string());

    match operation {
        OperationType::PutObject => {
            let write_object_data = storage::WriteObjectData {
                binary: body.into_data_stream(),
                path: object_path,
                mime_type,
                content_size,
                filename,
            };

            let result = storage::write_object(write_object_data).await;
            if let Err(e) = result {
                return Err(e.into());
            }

            return Ok(StatusCode::CREATED.into_response());
        }
        _ => {
            return Ok((StatusCode::BAD_REQUEST, "Unknown operation type").into_response());
        }
    }
}