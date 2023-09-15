use basicauth::AuthCheckResult;
use crate::types::WrapperState;
use axum::extract::State;
use axum::http::{header, Request, StatusCode};
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::log::{info, warn};

#[tracing::instrument(skip(state, req, next))]
pub async fn auth<B>(
    State(state): State<Arc<WrapperState>>,
    mut req: Request<B>,
    next: Next<B>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    info!("Auth middleware invoked.");
    let token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|auth_header| auth_header.to_str().ok())
        .and_then(|auth_value| auth_value.strip_prefix("Bearer "))
        .map(|auth| auth.to_string());

    let Some(token) = token else {
        warn!("The request did not attach a token to the authorization header.");
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "error": "You didn't provide a bearer token."
            })),
        ));
    };

    info!("Got token from authorization header: '{token}'");

    let Some((prefix, key)) = token.split_once('#') else {
        warn!("The given token is not valid due to missing separator: '{token}'");
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "error": "Token is in invalid format (missing separator)."
            })),
        ));
    };

    match state.auth_manager.check_key(prefix, key) {
        AuthCheckResult::Valid => {
            info!("The given token has been validated, prefix is '{prefix}'");
            req.extensions_mut().insert(prefix.to_owned());
            Ok(next.run(req).await)
        }
        AuthCheckResult::NoPrefixOrKeyFound => {
            info!("The given token is either not valid, or the key doesn't exist.");

            Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error": "Token is invalid or the key doesn't exist."
                })),
            ))
        }
        AuthCheckResult::ExpiredKey => {
            info!("The given token has expired, prefix is '{prefix}'");

            Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error": "Token is expired."
                })),
            ))
        }
    }
}
