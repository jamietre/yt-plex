use axum::{
    extract::{FromRequestParts, Query, State},
    http::{header, request::Parts, StatusCode},
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tracing::warn;

use crate::{auth as auth_util, AppState, DeviceCodeEntry};

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

// ── Device code initiation ────────────────────────────────────────────────────

#[derive(Deserialize)]
struct GoogleDeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_url: String,
    expires_in: u64,
    interval: u64,
}

#[derive(Serialize)]
pub struct DeviceLoginResponse {
    pub poll_token: String,
    pub user_code: String,
    pub verification_url: String,
    pub expires_in: u64,
    pub interval: u64,
}

pub async fn device_login(State(state): State<AppState>) -> axum::response::Response {
    let client_id = state.config.read().unwrap().google_oauth.client_id.clone();

    let resp = match state
        .http_client
        .post("https://oauth2.googleapis.com/device/code")
        .form(&[("client_id", client_id.as_str()), ("scope", "email")])
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            warn!("device code request failed: {e}");
            return (StatusCode::BAD_GATEWAY, "Failed to contact Google.").into_response();
        }
    };

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        warn!("device code request returned HTTP {status}: {body}");
        return (StatusCode::BAD_GATEWAY, "Failed to start sign-in.").into_response();
    }

    let google_resp: GoogleDeviceCodeResponse = match resp.json().await {
        Ok(d) => d,
        Err(e) => {
            warn!("device code parse failed: {e}");
            return (StatusCode::BAD_GATEWAY, "Unexpected response from Google.").into_response();
        }
    };

    let poll_token = auth_util::generate_token();
    {
        let mut states = state.oauth_states.lock().unwrap();
        states.insert(
            poll_token.clone(),
            DeviceCodeEntry {
                google_device_code: google_resp.device_code,
                expires_at: Instant::now() + Duration::from_secs(google_resp.expires_in),
                interval: google_resp.interval,
            },
        );
    }

    Json(DeviceLoginResponse {
        poll_token,
        user_code: google_resp.user_code,
        verification_url: google_resp.verification_url,
        expires_in: google_resp.expires_in,
        interval: google_resp.interval,
    })
    .into_response()
}

// ── Device code polling ───────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct PollParams {
    pub token: String,
}

#[derive(Serialize)]
pub struct PollResponse {
    pub status: &'static str, // "pending" | "done" | "denied" | "expired" | "error"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

// Google token endpoint response (device flow)
#[derive(Deserialize)]
struct GoogleTokenResponse {
    access_token: Option<String>,
    error: Option<String>,
    interval: Option<u64>,
}

#[derive(Deserialize)]
struct UserInfo {
    email: String,
}

pub async fn device_poll(
    State(state): State<AppState>,
    Query(params): Query<PollParams>,
) -> axum::response::Response {
    // Prune expired entries
    {
        let mut states = state.oauth_states.lock().unwrap();
        states.retain(|_, e| e.expires_at > Instant::now());
    }

    let (client_id, client_secret) = {
        let cfg = state.config.read().unwrap();
        (
            cfg.google_oauth.client_id.clone(),
            cfg.google_oauth.client_secret.clone(),
        )
    };

    let (google_device_code, stored_interval) = {
        let states = state.oauth_states.lock().unwrap();
        match states.get(&params.token) {
            None => {
                return Json(PollResponse {
                    status: "expired",
                    interval: None,
                    message: None,
                })
                .into_response();
            }
            Some(entry) => (entry.google_device_code.clone(), entry.interval),
        }
    };

    // Poll Google's token endpoint
    let token_resp = match state
        .http_client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("device_code", google_device_code.as_str()),
            (
                "grant_type",
                "urn:ietf:params:oauth:grant-type:device_code",
            ),
        ])
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            warn!("token poll request failed: {e}");
            return Json(PollResponse {
                status: "error",
                interval: None,
                message: Some("Failed to contact Google.".into()),
            })
            .into_response();
        }
    };

    let google_token: GoogleTokenResponse = match token_resp.json().await {
        Ok(d) => d,
        Err(e) => {
            warn!("token poll parse failed: {e}");
            return Json(PollResponse {
                status: "error",
                interval: None,
                message: Some("Unexpected response from Google.".into()),
            })
            .into_response();
        }
    };

    match google_token.error.as_deref() {
        Some("authorization_pending") => {
            return Json(PollResponse {
                status: "pending",
                interval: None,
                message: None,
            })
            .into_response();
        }
        Some("slow_down") => {
            return Json(PollResponse {
                status: "pending",
                interval: Some(google_token.interval.unwrap_or(stored_interval)),
                message: None,
            })
            .into_response();
        }
        Some("access_denied") => {
            state.oauth_states.lock().unwrap().remove(&params.token);
            return Json(PollResponse {
                status: "denied",
                interval: None,
                message: Some("Access was denied.".into()),
            })
            .into_response();
        }
        Some(other) => {
            warn!("Google token error: {other}");
            state.oauth_states.lock().unwrap().remove(&params.token);
            return Json(PollResponse {
                status: "error",
                interval: None,
                message: Some(format!("Google error: {other}")),
            })
            .into_response();
        }
        None => {} // fall through to access_token handling
    }

    let access_token = match google_token.access_token {
        Some(t) => t,
        None => {
            warn!("token poll returned neither access_token nor error");
            return Json(PollResponse {
                status: "error",
                interval: None,
                message: Some("Unexpected response from Google.".into()),
            })
            .into_response();
        }
    };

    // Fetch user's email
    let userinfo_resp = match state
        .http_client
        .get("https://www.googleapis.com/oauth2/v2/userinfo")
        .bearer_auth(&access_token)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            warn!("userinfo request failed: {e}");
            return Json(PollResponse {
                status: "error",
                interval: None,
                message: Some("Failed to fetch user info from Google.".into()),
            })
            .into_response();
        }
    };

    if !userinfo_resp.status().is_success() {
        let status = userinfo_resp.status();
        let body = userinfo_resp.text().await.unwrap_or_default();
        warn!("userinfo returned HTTP {status}: {body}");
        return Json(PollResponse {
            status: "error",
            interval: None,
            message: Some("Failed to fetch user info.".into()),
        })
        .into_response();
    }

    let userinfo: UserInfo = match userinfo_resp.json().await {
        Ok(d) => d,
        Err(e) => {
            warn!("userinfo parse failed: {e}");
            return Json(PollResponse {
                status: "error",
                interval: None,
                message: Some("Unexpected user info response.".into()),
            })
            .into_response();
        }
    };

    // Check against admin list
    let is_admin = {
        let cfg = state.config.read().unwrap();
        cfg.auth
            .admin_emails
            .iter()
            .any(|e| e.eq_ignore_ascii_case(&userinfo.email))
    };

    if !is_admin {
        warn!("device flow login rejected for {}", userinfo.email);
        state.oauth_states.lock().unwrap().remove(&params.token);
        return Json(PollResponse {
            status: "denied",
            interval: None,
            message: Some("Your account is not authorised.".into()),
        })
        .into_response();
    }

    // Create session
    let session_token = auth_util::generate_token();
    if let Err(e) = state.db.insert_session(&session_token) {
        warn!("insert session error: {e}");
        return Json(PollResponse {
            status: "error",
            interval: None,
            message: Some("Server error creating session.".into()),
        })
        .into_response();
    }

    // Clean up state
    state.oauth_states.lock().unwrap().remove(&params.token);

    let cookie = format!(
        "session={}; Path=/; HttpOnly; SameSite=Lax; Max-Age=604800",
        session_token
    );
    (
        StatusCode::OK,
        [(header::SET_COOKIE, cookie)],
        Json(PollResponse {
            status: "done",
            interval: None,
            message: None,
        }),
    )
        .into_response()
}

// ── Logout ────────────────────────────────────────────────────────────────────

pub async fn logout(
    State(state): State<AppState>,
    SessionToken(token): SessionToken,
) -> impl IntoResponse {
    if let Some(t) = token {
        let _ = state.db.delete_session(&t);
    }
    let clear = "session=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0";
    (StatusCode::OK, [(header::SET_COOKIE, clear)], "OK")
}
