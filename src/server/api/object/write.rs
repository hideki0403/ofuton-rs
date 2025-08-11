use axum::{body::Body, extract::Request, http::{Method, Response, StatusCode}, response::IntoResponse};
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

// S3 API Response Structures
#[derive(Debug, Serialize)]
#[serde(rename = "InitiateMultipartUploadResult")]
pub struct S3InitiateMultipartUploadResult {
    #[serde(rename = "Bucket")]
    pub bucket: String,
    #[serde(rename = "Key")]
    pub key: String,
    #[serde(rename = "UploadId")]
    pub upload_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename = "CompleteMultipartUploadResult")]
pub struct S3CompleteMultipartUploadResult {
    #[serde(rename = "Location")]
    pub location: String,
    #[serde(rename = "Bucket")]
    pub bucket: String,
    #[serde(rename = "Key")]
    pub key: String,
    #[serde(rename = "ETag")]
    pub e_tag: String,
}

pub async fn write_handler(request: Request<Body>) -> AppResult<impl IntoResponse> {
    let object_path = request.uri().path().to_string();
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
    let content_disposition = parse_content_disposition(get_header(&parts.headers, "Content-Disposition", None).as_str());

    match operation {
        OperationType::PutObject => {
            let write_object_data = storage::WriteObjectData {
                binary: body.into_data_stream(),
                path: object_path,
                mime_type,
                content_size,
                filename: content_disposition.filename.clone().unwrap_or("unknown".to_string()),
                encoded_filename: content_disposition.encoded_filename.clone(),
            };

            let result = storage::put_object(write_object_data).await;
            if let Err(e) = result {
                return Err(e.into());
            }

            return Ok(StatusCode::CREATED.into_response());
        }
        OperationType::CreateMultipartUpload => {
            let upload_id = storage::create_multipart_upload(object_path.clone(), content_disposition.filename.unwrap_or("unknown".to_string()), content_disposition.encoded_filename, mime_type);

            let (bucket, key) = object_path.split_once('/').unwrap_or(("", &object_path));
            let response = S3InitiateMultipartUploadResult {
                bucket: bucket.to_string(),
                key: key.to_string(),
                upload_id,
            };

            let xml_response = serde_xml_rs::to_string(&response);
            if let Err(e) = xml_response {
                return Err(e.into());
            }

            let mut response = Response::new(xml_response.unwrap());
            response.headers_mut().insert("Content-Type", "application/xml".parse().unwrap());

            return Ok(response.into_response());
        }
        OperationType::UploadPart => {
            if multipart_upload_state.upload_id.is_none() || multipart_upload_state.part_number.is_none() {
                return Ok((StatusCode::BAD_REQUEST, "Missing uploadId or partNumber").into_response());
            }

            if !multipart_upload_state.is_registered {
                return Ok((StatusCode::BAD_REQUEST, "Invalid or expires uploadId").into_response());
            }

            let upload_id = multipart_upload_state.upload_id.as_ref().unwrap();
            let part_number = multipart_upload_state.part_number.unwrap();
            storage::upload_part(upload_id.clone(), part_number, body.into_data_stream()).await?;

            let response = Response::builder()
                .status(StatusCode::OK)
                .header("ETag", Uuid::new_v4().to_string())
                .body(Body::empty())
                .unwrap();

            return Ok(response);
        }
        OperationType::CompleteMultipartUpload => {
            let upload_id = multipart_upload_state.upload_id.clone();
            if upload_id.is_none() {
                return Ok((StatusCode::BAD_REQUEST, "Missing uploadId").into_response());
            }

            if !multipart_upload_state.is_registered {
                return Ok((StatusCode::BAD_REQUEST, "Invalid or expired uploadId").into_response());
            }

            storage::complete_multipart_upload(upload_id.unwrap()).await?;

            let location = parts.uri.to_string();
            let (bucket, key) = object_path.split_once('/').unwrap_or(("", &object_path));
            let response = S3CompleteMultipartUploadResult {
                location: location.split_once('?').map_or(location.clone(), |(loc, _)| loc.to_string()),
                bucket: bucket.to_string(),
                key: key.to_string(),
                e_tag: Uuid::new_v4().to_string(),
            };

            let xml_response = serde_xml_rs::to_string(&response);
            if let Err(e) = xml_response {
                return Err(e.into());
            }

            let mut response = Response::new(xml_response.unwrap());
            response.headers_mut().insert("Content-Type", "application/xml".parse().unwrap());

            return Ok(response.into_response());
        }
        OperationType::AbortMultipartUpload => {
            let upload_id = multipart_upload_state.upload_id.clone();
            if upload_id.is_none() {
                return Ok((StatusCode::BAD_REQUEST, "Missing uploadId").into_response());
            }

            storage::abort_multipart_upload(upload_id.unwrap()).await?;
            return Ok(StatusCode::NO_CONTENT.into_response());
        }
        OperationType::DeleteObject => {
            let result = storage::delete_object(object_path).await;
            if let Err(e) = result {
                return Err(e.into());
            }

            return Ok(StatusCode::NO_CONTENT.into_response());
        }
        _ => {
            return Ok((StatusCode::BAD_REQUEST, "Unknown operation type").into_response());
        }
    }
}