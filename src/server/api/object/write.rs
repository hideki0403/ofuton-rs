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

pub async fn write_handler(Path(object_path): Path<String>, Query(params): Query<ReqParams>) -> AppResult<impl IntoResponse> {
    if object_path.is_empty() {
        return Ok(StatusCode::BAD_REQUEST.into_response());
    }

    if params.x_id.is_none() {
        return Ok((StatusCode::BAD_REQUEST, "x-id is required").into_response());
    }

    match params.x_id.unwrap().as_str() {
        "PutObject" => {
            // TODO
        }
        _ => {
            return Ok((StatusCode::BAD_REQUEST, "Unknown operation type").into_response());
        }
    }

    return Ok((StatusCode::NOT_IMPLEMENTED, "write_handler is not implemented yet").into_response());
}