//! A middleware responsible for ensuring the term is valid.

use std::sync::Arc;

use axum::extract::{Path, State, Request};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::{json, Value};
use tracing::info;

use crate::types::WrapperState;

/// A middleware function that validates a term that's passed as part of the path
/// is supported by the server.
#[tracing::instrument(skip(state, req, next))]
pub async fn validate_term(
    Path(term): Path<String>,
    State(state): State<Arc<WrapperState>>,
    req: Request,
    next: Next,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    info!("Validating if term is supported.");
    let term = term.to_uppercase();
    if state.all_terms.contains_key(&term) {
        Ok(next.run(req).await)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "The specified term cannot be found"
            })),
        ))
    }
}
