use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::{json, Value};
use tracing::log::info;

use crate::types::WrapperState;

/// A middleware function that checks if the wrapper is able to handle requests.
#[tracing::instrument(skip(state, req, next))]
pub async fn validate_wrapper_running(
    State(state): State<Arc<WrapperState>>,
    req: Request,
    next: Next,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    info!("Validating if API is ready.");
    if state.is_running() {
        Ok(next.run(req).await)
    } else {
        Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "The API isn't ready to make requests at this time."
            })),
        ))
    }
}
