use std::path;
use axum::{extract::{Path, Query}, http::{HeaderMap, Method, StatusCode}, response::IntoResponse};
use axum_range::{Ranged, KnownSize};
use axum_extra::{headers::Range, TypedHeader};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::Deserialize;
use tokio::fs::File;
use crate::{config, server::AppError};
use crate::server::AppResult;
use crate::database;
use crate::entity;

#[derive(Deserialize, Debug)]
pub struct ReqParams {
    #[serde(alias = "x-id")]
    x_id: Option<String>,
    #[serde(alias = "uploadId")]
    upload_id: Option<String>,
    #[serde(alias = "partNumber")]
    part_number: Option<u32>,
}

#[derive(PartialEq)]
enum OperationType {
    PutObject,
    CreateMultipartUpload,
    UploadPart,
    CompleteMultipartUpload,
    AbortMultipartUpload,
    DeleteObject,
    Unknown,
}

pub async fn write_handler(Path(object_path): Path<String>, Query(params): Query<ReqParams>, request: Request<Body>) -> AppResult<impl IntoResponse> {
    if object_path.is_empty() {
        return Ok(StatusCode::BAD_REQUEST.into_response());
    }

    let operation = match params.x_id.unwrap_or_default().as_str() {
        "PutObject" => OperationType::PutObject,
        "CreateMultipartUpload" => OperationType::CreateMultipartUpload,
        "UploadPart" => OperationType::UploadPart,
        "CompleteMultipartUpload" => OperationType::CompleteMultipartUpload,
        "AbortMultipartUpload" => OperationType::AbortMultipartUpload,
        "DeleteObject" => OperationType::DeleteObject,
        _ => {
            match *request.method() {
                Method::PUT => {
                    if params.upload_id.is_some() {
                        OperationType::UploadPart
                    } else {
                        OperationType::PutObject
                    }
                }
                Method::POST => {
                    if params.upload_id.is_some() {
                        OperationType::CompleteMultipartUpload
                    } else {
                        OperationType::CreateMultipartUpload
                    }
                }
                Method::DELETE => {
                    if params.upload_id.is_some() {
                        OperationType::AbortMultipartUpload
                    } else {
                        OperationType::DeleteObject
                    }
                },
                _ => OperationType::Unknown,
            }
        },
    };

    if operation == OperationType::Unknown {
        return Ok((StatusCode::BAD_REQUEST, "unknown operation").into_response());
    }

    let mime_type = get_header(&request, "Content-Type", Some("application/octet-stream".to_string()));
    let content_size = get_header(&request, "Content-Length", None).parse::<i64>().unwrap_or(0);
    let filename = parse_content_disposition(get_header(&request, "Content-Disposition", None).as_str()).unwrap_or("unknown".to_string());
    let (_, body) = request.into_parts();

    match operation {
        OperationType::PutObject => {
            // TODO
        }
        _ => {
            return Ok((StatusCode::BAD_REQUEST, "Unknown operation type").into_response());
        }
    }
}