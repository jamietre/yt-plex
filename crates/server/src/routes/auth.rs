use axum::{
    extract::{FromRequestParts, State},
    http::{header, request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use tracing::warn;

use crate::{auth as auth_util, AppState};

/// Extracts the raw session token string from the Cookie header (or None).
pub struct SessionToken(pub Option<String>);

impl<S> FromRequestParts<S> for SessionToken
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get(header::COOKIE)
            .and_then(|v| v.to_str().ok())
            .and_then(|cookies| {
                cookies
                    .split(';')
                    .map(|c| c.trim())
                    .find(|c| c.starts_with("session="))
                    .map(|c| c["session=".len()..].to_string())
            });
        Ok(SessionToken(token))
    }
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub password: String,
}

pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Response {
    let hash = state.config.read().unwrap().auth.admin_password_hash.clone();
    match auth_util::verify_password(&body.password, &hash) {
        Ok(true) => {}
        Ok(false) => {
            warn!("failed login attempt");
            return (StatusCode::UNAUTHORIZED, "Invalid password").into_response();
        }
        Err(e) => {
            warn!("password verify error: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Server error").into_response();
        }
    }

    let token = auth_util::generate_token();
    if let Err(e) = state.db.insert_session(&token) {
        warn!("insert session error: {e}");
        return (StatusCode::INTERNAL_SERVER_ERROR, "Server error").into_response();
    }

    let cookie = format!(
        "session={}; Path=/; HttpOnly; SameSite=Strict; Max-Age=604800",
        token
    );
    (StatusCode::OK, [(header::SET_COOKIE, cookie)], "OK").into_response()
}

pub async fn logout(State(state): State<AppState>, SessionToken(token): SessionToken) -> Response {
    if let Some(t) = token {
        let _ = state.db.delete_session(&t);
    }
    let clear = "session=; Path=/; HttpOnly; Max-Age=0".to_string();
    (StatusCode::OK, [(header::SET_COOKIE, clear)], "OK").into_response()
}
