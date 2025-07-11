use axum::{body::Body, extract::Query, http::Request, middleware::Next, response::Response};
use serde::Deserialize;
use crate::storage;

#[derive(Deserialize, Debug)]
pub struct ReqParams {
    #[serde(alias = "uploadId")]
    upload_id: Option<String>,
    #[serde(alias = "partNumber")]
    part_number: Option<u16>,
}

#[derive(Clone)]
pub struct MultipartUploadState {
    pub is_registered: bool,
    pub upload_id: Option<String>,
    pub part_number: Option<u16>,
}

pub async fn multipart_state_manager(Query(params): Query<ReqParams>, mut request: Request<Body>, next: Next) -> Response {
    let upload_id = params.upload_id.clone();
    let is_registered = upload_id.is_some() && storage::MULTIPART_UPLOAD_STATE.lock().unwrap().contains_key(upload_id.as_ref().unwrap());

    let state = MultipartUploadState {
        is_registered,
        upload_id,
        part_number: params.part_number,
    };

    request.extensions_mut().insert(state);
    return next.run(request).await;
}
