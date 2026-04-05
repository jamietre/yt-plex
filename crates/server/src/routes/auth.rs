use axum::{
    body::Body,
    extract::{FromRequestParts, Query, State},
    http::{header, request::Parts, Response, StatusCode},
    response::{IntoResponse, Redirect},
};
use serde::Deserialize;
use std::time::{Duration, Instant};
use tracing::warn;

use crate::{auth as auth_util, AppState};

const STATE_TTL: Duration = Duration::from_secs(600);

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

pub async fn oauth_login(State(state): State<AppState>) -> impl IntoResponse {
    let oauth_state = auth_util::generate_token();
    {
        let mut states = state.oauth_states.lock().unwrap();
        states.insert(oauth_state.clone(), Instant::now());
    }
    let cfg = state.config.read().unwrap();
    let url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth\
         ?response_type=code\
         &client_id={}\
         &redirect_uri={}\
         &scope=email\
         &state={}",
        urlencoding::encode(&cfg.google_oauth.client_id),
        urlencoding::encode(&cfg.google_oauth.redirect_uri),
        oauth_state, // hex string, no encoding needed
    );
    Redirect::to(&url)
}

#[derive(Deserialize)]
pub struct CallbackParams {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct UserInfo {
    email: String,
}

pub async fn oauth_callback(
    State(state): State<AppState>,
    Query(params): Query<CallbackParams>,
) -> axum::response::Response {
    // Prune expired states on every callback
    {
        let mut states = state.oauth_states.lock().unwrap();
        states.retain(|_, t| t.elapsed() < STATE_TTL);
    }

    if let Some(err) = params.error {
        warn!("OAuth error from Google: {err}");
        return error_page("Google sign-in was cancelled or failed.");
    }

    let code = match params.code {
        Some(c) => c,
        None => return error_page("Missing code parameter."),
    };
    let incoming_state = match params.state {
        Some(s) => s,
        None => return error_page("Missing state parameter."),
    };

    // Validate and consume state (prevents CSRF)
    let valid = {
        let mut states = state.oauth_states.lock().unwrap();
        states
            .remove(&incoming_state)
            .map(|t| t.elapsed() < STATE_TTL)
            .unwrap_or(false)
    };
    if !valid {
        return error_page("Invalid or expired state. Please try again.");
    }

    let (client_id, client_secret, redirect_uri) = {
        let cfg = state.config.read().unwrap();
        (
            cfg.google_oauth.client_id.clone(),
            cfg.google_oauth.client_secret.clone(),
            cfg.google_oauth.redirect_uri.clone(),
        )
    };

    // Exchange code for access token
    let http = reqwest::Client::new();
    let token_resp = match http
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
        ])
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            warn!("token exchange request failed: {e}");
            return error_page("Failed to contact Google. Please try again.");
        }
    };

    let token_data: TokenResponse = match token_resp.json().await {
        Ok(d) => d,
        Err(e) => {
            warn!("token exchange parse failed: {e}");
            return error_page("Unexpected response from Google. Please try again.");
        }
    };

    // Fetch user's email from userinfo endpoint
    let userinfo_resp = match http
        .get("https://www.googleapis.com/oauth2/v2/userinfo")
        .bearer_auth(&token_data.access_token)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            warn!("userinfo request failed: {e}");
            return error_page("Failed to fetch user info from Google.");
        }
    };

    let userinfo: UserInfo = match userinfo_resp.json().await {
        Ok(d) => d,
        Err(e) => {
            warn!("userinfo parse failed: {e}");
            return error_page("Unexpected user info response from Google.");
        }
    };

    // Check email against configured admin list
    let is_admin = {
        let cfg = state.config.read().unwrap();
        cfg.auth
            .admin_emails
            .iter()
            .any(|e| e.eq_ignore_ascii_case(&userinfo.email))
    };

    if !is_admin {
        warn!("OAuth login rejected for {}", userinfo.email);
        return (
            StatusCode::FORBIDDEN,
            axum::response::Html(
                "<h1>Access denied</h1>\
                 <p>Your account is not authorised to access this application.</p>\
                 <p><a href=\"/api/auth/login\">Try a different account</a></p>",
            ),
        )
            .into_response();
    }

    // Create session and set cookie
    let token = auth_util::generate_token();
    if let Err(e) = state.db.insert_session(&token) {
        warn!("insert session error: {e}");
        return error_page("Server error creating session.");
    }

    let cookie = format!(
        "session={}; Path=/; HttpOnly; SameSite=Lax; Max-Age=604800",
        token
    );
    Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header(header::LOCATION, "/")
        .header(header::SET_COOKIE, cookie)
        .body(Body::empty())
        .unwrap()
}

pub async fn logout(State(state): State<AppState>, SessionToken(token): SessionToken) -> impl IntoResponse {
    if let Some(t) = token {
        let _ = state.db.delete_session(&t);
    }
    let clear = "session=; Path=/; HttpOnly; Max-Age=0";
    (StatusCode::OK, [(header::SET_COOKIE, clear)], "OK")
}

fn error_page(msg: &str) -> axum::response::Response {
    let html = format!(
        "<h1>Sign-in failed</h1>\
         <p>{msg}</p>\
         <p><a href=\"/api/auth/login\">Try again</a></p>"
    );
    (StatusCode::BAD_REQUEST, axum::response::Html(html)).into_response()
}
