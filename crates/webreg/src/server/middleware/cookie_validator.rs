use axum::extract::Request;
use axum::http::header::COOKIE;
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::{json, Value};
use tracing::log::info;

/// A middleware function that checks if the wrapper is able to handle requests.
#[tracing::instrument(skip(req, next))]
pub async fn check_cookies(
    header_map: HeaderMap,
    req: Request,
    next: Next,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    info!("Validating if cookie header is available.");
    if let Some(header) = header_map.get(COOKIE) {
        match header.to_str() {
            Ok(_) => Ok(next.run(req).await),
            Err(_) => Err((
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "Your cookies must only contain ASCII characters."
                })),
            )),
        }
    } else {
        Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "You must provide your WebReg cookies for this endpoint."
            })),
        ))
    }
}
