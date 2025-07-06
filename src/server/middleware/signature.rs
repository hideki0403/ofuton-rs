use std::{collections::HashMap, sync::Arc};
use axum::{body::Body, extract::State, http::{Request, Uri}, middleware::Next, response::Response};
use sha2::{Sha256, Digest};
use hmac::{Hmac, Mac};
use regex::Regex;

#[derive(Clone)]
pub struct SignatureVerificationState {
    pub key: Arc<String>,
    pub secret: Arc<String>,
}

type HmacSha256 = Hmac<Sha256>;

pub async fn signature_verification(State(signatures): State<SignatureVerificationState>, request: Request<Body>, next: Next) -> Response {
    if !internal_verify(&request, &signatures.key, &signatures.secret) {
        return Response::builder()
            .status(403)
            .body(Body::from("Forbidden: Invalid signature"))
            .unwrap();
    }

    return next.run(request).await;
}

fn internal_verify(request: &Request<Body>, access_key: &str, secret_key: &str) -> bool {
    let authorization = get_header(request, "Authorization", None);
    if authorization.is_empty() {
        tracing::debug!("SignatureVerification Failed: Authorization header is missing or empty");
        return false;
    }

    let components = get_components(authorization.as_str());
    let signature = components.get("Signature").unwrap_or(&"");
    let credentials = components.get("Credential").unwrap_or(&"").split("/").collect::<Vec<&str>>();

    if signature.is_empty() || credentials.len() != 5 {
        tracing::debug!("SignatureVerification Failed: Invalid signature or credentials length mismatch");
        return false;
    }

    if credentials[0] != access_key {
        tracing::debug!("SignatureVerification Failed: Access key mismatch");
        return false;
    }

    let signed_headers = components.get("SignedHeaders").unwrap_or(&"").split(';').collect::<Vec<&str>>();
    let string_to_sign = get_string_to_sign(request, &credentials, &signed_headers);

    let mut mac = HmacSha256::new_from_slice(format!("AWS4{}", secret_key).as_bytes()).unwrap();
    mac.update(credentials[1].as_bytes()); // Date
    let date_key = mac.finalize().into_bytes();

    let mut mac = HmacSha256::new_from_slice(&date_key).unwrap();
    mac.update(credentials[2].as_bytes()); // Region
    let region_key = mac.finalize().into_bytes();

    let mut mac = HmacSha256::new_from_slice(&region_key).unwrap();
    mac.update(credentials[3].as_bytes()); // Service
    let service_key = mac.finalize().into_bytes();

    let mut mac = HmacSha256::new_from_slice(&service_key).unwrap();
    mac.update(credentials[4].as_bytes()); // "aws4_request"
    let signing_key = mac.finalize().into_bytes();

    let mut mac = HmacSha256::new_from_slice(&signing_key).unwrap();
    mac.update(string_to_sign.as_bytes());
    let signature_bytes = mac.finalize().into_bytes();

    let calculated_signature = format!("{:x}", signature_bytes);
    let verify_result = calculated_signature == *signature;
    if !verify_result {
        tracing::debug!("SignatureVerification Failed: Signature mismatch. Expected: {}, Got: {}", calculated_signature, signature);
    }

    return verify_result;
}

fn get_components(authorization: &str) -> HashMap<&str, &str> {
    let trimmed_authorization = authorization.split_once(' ');
    if trimmed_authorization.is_none() {
        return HashMap::new();
    }

    let components_str = trimmed_authorization.unwrap().1;
    return components_str
        .split(',')
        .filter_map(|s| s.trim().split_once('='))
        .collect();
}

fn get_query_string(uri: &Uri) -> String {
    let mut pairs = uri.query().unwrap_or("")
        .split('&')
        .filter_map(|s| s.split_once('=').map(|(k, v)| (k.to_string(), v.to_string())))
        .filter(|(k, _)| k != "X-Amz-Signature")
        .map(|(k, v)| format!("{}={}", urlencoding::encode(k.as_str()), urlencoding::encode(v.as_str())))
        .collect::<Vec<String>>();

    pairs.sort();
    return pairs.join("&");
}

fn get_string_to_sign(request: &Request<Body>, credentials: &Vec<&str>, signed_headers: &Vec<&str>) -> String {
    let canonical_headers = signed_headers
        .iter()
        .map(|h| {
            let header_value = get_header(request, *h, None);
            format!("{}:{}\n", h, header_value)
        })
        .collect::<Vec<String>>()
        .join("");

    let content_hash = get_header(request, "X-Amz-Content-Sha256", Some("UNSIGNED-PAYLOAD".to_string()));
    let uri_regex = Regex::new(r"\?.*").unwrap();

    let canonical_request_string = [
        request.method().as_str(),
        uri_regex.replace(&request.uri().to_string(), "").as_ref(),
        &get_query_string(&request.uri()),
        canonical_headers.as_str(),
        signed_headers.join(";").as_str(),
        content_hash.as_str(),
    ].join("\n");

    let mut hasher = Sha256::new();
    hasher.update(canonical_request_string.as_bytes());
    let canonical_request_hash = format!("{:x}", hasher.finalize());

    let credentials_scope = [
        credentials[1], // Date
        credentials[2], // Region
        credentials[3], // Service
        credentials[4], // "aws4_request"
    ].join("/");

    return [
        "AWS4-HMAC-SHA256",
        get_header(request, "X-Amz-Date", None).as_str(),
        credentials_scope.as_str(),
        canonical_request_hash.as_str(),
    ].join("\n");
}

fn get_header(request: &Request<Body>, header_name: &str, fallback: Option<String>) -> String {
    request.headers().get(header_name)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or(fallback.unwrap_or_default())
}