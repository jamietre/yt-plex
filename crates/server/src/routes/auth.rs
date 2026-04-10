use axum::{
    extract::{FromRequestParts, Query, State},
    http::{header, request::Parts, StatusCode},
    response::{IntoResponse, Redirect},
    Json,
};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tracing::warn;
use yt_plex_common::config::OAuthFlow;

use crate::{auth as auth_util, AppState, DeviceCodeEntry, ExchangeTokenEntry, OAuthStateEntry};

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

// ── Flow probe ───────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct AuthFlowResponse {
    flow: String,
}

/// GET /api/auth/flow — returns which login flow is configured.
pub async fn auth_flow(State(state): State<AppState>) -> impl IntoResponse {
    let flow = match state.config.read().unwrap().google_oauth.flow {
        OAuthFlow::Device => "device",
        OAuthFlow::AuthorizationCode => "authorization_code",
    };
    Json(AuthFlowResponse { flow: flow.to_string() })
}

// ── OAuth Authorization Code flow ────────────────────────────────────────────

#[derive(Deserialize)]
pub struct LoginParams {
    /// Origin the login was initiated from, e.g. "http://PRIVATE_IP_REDACTED:32113".
    /// Stored server-side so the callback can redirect back to the local app.
    pub return_to: Option<String>,
}

/// GET /api/auth/login — redirect browser to Google's consent screen.
pub async fn oauth_login(
    State(state): State<AppState>,
    Query(params): Query<LoginParams>,
) -> impl IntoResponse {
    let (client_id, redirect_uri) = {
        let cfg = state.config.read().unwrap();
        let uri = cfg.google_oauth.redirect_uri.clone().unwrap_or_default();
        (cfg.google_oauth.client_id.clone(), uri)
    };

    // Only accept http/https origins to prevent open-redirect abuse.
    let return_to = params.return_to.filter(|u| {
        u.starts_with("http://") || u.starts_with("https://")
    });

    let csrf = auth_util::generate_token();
    {
        let mut states = state.oauth_states.lock().unwrap();
        states.insert(csrf.clone(), OAuthStateEntry {
            expires_at: Instant::now() + Duration::from_secs(600),
            return_to,
        });
    }

    let url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth\
         ?client_id={}\
         &redirect_uri={}\
         &response_type=code\
         &scope=email\
         &state={}",
        urlencoding::encode(&client_id),
        urlencoding::encode(&redirect_uri),
        csrf,
    );

    Redirect::to(&url)
}

/// GET /api/auth/callback?code=…&state=… — Google redirects here after consent.
#[derive(Deserialize)]
pub struct CallbackParams {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
}

pub async fn oauth_callback(
    State(state): State<AppState>,
    Query(params): Query<CallbackParams>,
) -> impl IntoResponse {
    // User denied or Google returned an error
    if params.error.is_some() {
        return Redirect::to("/login?error=denied").into_response();
    }

    let (code, csrf) = match (params.code, params.state) {
        (Some(c), Some(s)) => (c, s),
        _ => return Redirect::to("/login?error=invalid").into_response(),
    };

    // Validate and consume the CSRF state
    let return_to = {
        let mut states = state.oauth_states.lock().unwrap();
        states.retain(|_, e| e.expires_at > Instant::now());
        match states.remove(&csrf) {
            Some(entry) => entry.return_to,
            None => return Redirect::to("/login?error=invalid").into_response(),
        }
    };

    let (client_id, client_secret, redirect_uri) = {
        let cfg = state.config.read().unwrap();
        (
            cfg.google_oauth.client_id.clone(),
            cfg.google_oauth.client_secret.clone(),
            cfg.google_oauth.redirect_uri.clone().unwrap_or_default(),
        )
    };

    // Exchange authorization code for access token
    #[derive(Deserialize)]
    struct TokenResponse {
        access_token: Option<String>,
        error: Option<String>,
    }

    let token_resp = match state
        .http_client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("code", code.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            warn!("token exchange failed: {e}");
            return Redirect::to("/login?error=server").into_response();
        }
    };

    let token: TokenResponse = match token_resp.json().await {
        Ok(t) => t,
        Err(e) => {
            warn!("token parse failed: {e}");
            return Redirect::to("/login?error=server").into_response();
        }
    };

    if let Some(err) = token.error {
        warn!("Google token error: {err}");
        return Redirect::to("/login?error=denied").into_response();
    }

    let access_token = match token.access_token {
        Some(t) => t,
        None => return Redirect::to("/login?error=server").into_response(),
    };

    // Fetch email
    #[derive(Deserialize)]
    struct UserInfo {
        email: String,
    }

    let userinfo: UserInfo = match state
        .http_client
        .get("https://www.googleapis.com/oauth2/v2/userinfo")
        .bearer_auth(&access_token)
        .send()
        .await {
        Ok(r) => match r.json().await {
            Ok(u) => u,
            Err(e) => {
                warn!("userinfo parse failed: {e}");
                return Redirect::to("/login?error=server").into_response();
            }
        },
        Err(e) => {
            warn!("userinfo request failed: {e}");
            return Redirect::to("/login?error=server").into_response();
        }
    };

    // Check admin list
    let is_admin = {
        let cfg = state.config.read().unwrap();
        cfg.auth
            .admin_emails
            .iter()
            .any(|e| e.eq_ignore_ascii_case(&userinfo.email))
    };

    if !is_admin {
        warn!("OAuth login rejected for {}", userinfo.email);
        return Redirect::to("/login?error=denied").into_response();
    }

    // Create session
    let session_token = auth_util::generate_token();
    if let Err(e) = state.db.insert_session(&session_token) {
        warn!("insert session error: {e}");
        return Redirect::to("/login?error=server").into_response();
    }

    // Auto-create or look up admin profile linked to this email
    let admin_profile_id: Option<i64> = match state.db.get_profile_by_email(&userinfo.email) {
        Ok(Some(p)) => Some(p.id),
        Ok(None) => {
            match state
                .db
                .create_profile(&userinfo.email, Some(&userinfo.email), true)
            {
                Ok(p) => Some(p.id),
                Err(e) => {
                    warn!("create admin profile for {}: {e}", userinfo.email);
                    None
                }
            }
        }
        Err(e) => {
            warn!("get_profile_by_email for {}: {e}", userinfo.email);
            None
        }
    };

    if let Some(profile_id) = admin_profile_id {
        if let Err(e) = state.db.set_session_profile(&session_token, profile_id) {
            warn!("set_session_profile: {e}");
        }
    }

    // If login was initiated from a different origin (e.g. local app through a
    // public callback URL), issue a short-lived exchange token and redirect back
    // to the local app to set the cookie on the correct domain.
    if let Some(origin) = return_to {
        let exchange = auth_util::generate_token();
        {
            let mut tokens = state.exchange_tokens.lock().unwrap();
            tokens.retain(|_, e: &mut ExchangeTokenEntry| e.expires_at > Instant::now());
            tokens.insert(exchange.clone(), ExchangeTokenEntry {
                session_token,
                profile_id: admin_profile_id,
                expires_at: Instant::now() + Duration::from_secs(30),
            });
        }
        return Redirect::to(&format!("{}/api/auth/exchange?t={}", origin, exchange)).into_response();
    }

    // Same-origin flow: set cookies directly.
    let session_cookie = format!(
        "session={}; Path=/; HttpOnly; SameSite=Lax; Max-Age=604800",
        session_token
    );
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(header::SET_COOKIE, session_cookie.parse().expect("valid cookie"));
    if let Some(profile_id) = admin_profile_id {
        let profile_cookie = format!("yt_plex_profile={profile_id}; Path=/; SameSite=Lax; Max-Age=604800");
        headers.append(header::SET_COOKIE, profile_cookie.parse().expect("valid cookie"));
    }
    (headers, Redirect::to("/")).into_response()
}

// ── Exchange token (bridges public callback → local session cookie) ───────────

#[derive(Deserialize)]
pub struct ExchangeParams {
    pub t: String,
}

/// GET /api/auth/exchange?t=… — redeem a short-lived token for a session cookie.
/// Only reachable on the local network; sets the cookie on the local domain.
pub async fn exchange(
    State(state): State<AppState>,
    Query(params): Query<ExchangeParams>,
) -> impl IntoResponse {
    let entry = {
        let mut tokens = state.exchange_tokens.lock().unwrap();
        tokens.retain(|_, e| e.expires_at > Instant::now());
        tokens.remove(&params.t)
    };

    let entry = match entry {
        Some(e) => e,
        None => return Redirect::to("/login?error=invalid").into_response(),
    };

    let session_cookie = format!(
        "session={}; Path=/; HttpOnly; SameSite=Lax; Max-Age=604800",
        entry.session_token
    );
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(header::SET_COOKIE, session_cookie.parse().expect("valid cookie"));
    if let Some(profile_id) = entry.profile_id {
        let profile_cookie = format!("yt_plex_profile={profile_id}; Path=/; SameSite=Lax; Max-Age=604800");
        headers.append(header::SET_COOKIE, profile_cookie.parse().expect("valid cookie"));
    }
    (headers, Redirect::to("/")).into_response()
}

// ── Device authorization grant flow ──────────────────────────────────────────

#[derive(Serialize)]
pub struct DeviceStartResponse {
    pub poll_token: String,
    pub user_code: String,
    pub verification_url: String,
    pub interval: u64,
    pub expires_in: u64,
}

/// POST /api/auth/device — start a device-flow login.
pub async fn device_start(State(state): State<AppState>) -> impl IntoResponse {
    let (client_id, client_secret) = {
        let cfg = state.config.read().unwrap();
        (cfg.google_oauth.client_id.clone(), cfg.google_oauth.client_secret.clone())
    };

    #[derive(Deserialize)]
    struct DeviceCodeResponse {
        device_code: Option<String>,
        user_code: Option<String>,
        verification_url: Option<String>,
        interval: Option<u64>,
        expires_in: Option<u64>,
        error: Option<String>,
    }

    let resp = match state
        .http_client
        .post("https://oauth2.googleapis.com/device/code")
        .form(&[
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("scope", "email"),
        ])
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            warn!("device code request failed: {e}");
            return StatusCode::BAD_GATEWAY.into_response();
        }
    };

    let data: DeviceCodeResponse = match resp.json().await {
        Ok(d) => d,
        Err(e) => {
            warn!("device code parse failed: {e}");
            return StatusCode::BAD_GATEWAY.into_response();
        }
    };

    if let Some(err) = data.error {
        warn!("Google device code error: {err}");
        return StatusCode::BAD_GATEWAY.into_response();
    }

    let (device_code, user_code, verification_url) = match (data.device_code, data.user_code, data.verification_url) {
        (Some(d), Some(u), Some(v)) => (d, u, v),
        _ => {
            warn!("incomplete device code response");
            return StatusCode::BAD_GATEWAY.into_response();
        }
    };

    let expires_in = data.expires_in.unwrap_or(1800);
    let interval = data.interval.unwrap_or(5);
    let poll_token = auth_util::generate_token();

    {
        let mut codes = state.device_codes.lock().unwrap();
        codes.retain(|_, e| e.expires_at > Instant::now());
        codes.insert(poll_token.clone(), DeviceCodeEntry {
            device_code,
            expires_at: Instant::now() + Duration::from_secs(expires_in),
        });
    }

    Json(DeviceStartResponse { poll_token, user_code, verification_url, interval, expires_in }).into_response()
}

#[derive(Deserialize)]
pub struct PollParams {
    pub poll_token: String,
}

#[derive(Serialize)]
pub struct PollResponse {
    pub status: String, // "pending" | "done" | "expired"
}

/// POST /api/auth/poll — poll for device-flow completion.
pub async fn device_poll(
    State(state): State<AppState>,
    Json(params): Json<PollParams>,
) -> impl IntoResponse {
    let entry = {
        let codes = state.device_codes.lock().unwrap();
        codes.get(&params.poll_token).cloned()
    };

    let entry = match entry {
        Some(e) if e.expires_at > Instant::now() => e,
        Some(_) => return Json(PollResponse { status: "expired".into() }).into_response(),
        None    => return Json(PollResponse { status: "expired".into() }).into_response(),
    };

    let (client_id, client_secret) = {
        let cfg = state.config.read().unwrap();
        (cfg.google_oauth.client_id.clone(), cfg.google_oauth.client_secret.clone())
    };

    #[derive(Deserialize)]
    struct TokenResponse {
        access_token: Option<String>,
        error: Option<String>,
    }

    let token_resp = match state
        .http_client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("device_code", entry.device_code.as_str()),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ])
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            warn!("device poll token request failed: {e}");
            return StatusCode::BAD_GATEWAY.into_response();
        }
    };

    let token: TokenResponse = match token_resp.json().await {
        Ok(t) => t,
        Err(e) => {
            warn!("device poll token parse failed: {e}");
            return StatusCode::BAD_GATEWAY.into_response();
        }
    };

    // authorization_pending or slow_down — not an error, just not ready yet
    if let Some(ref err) = token.error {
        if err == "authorization_pending" || err == "slow_down" {
            return Json(PollResponse { status: "pending".into() }).into_response();
        }
        warn!("device poll error: {err}");
        return Json(PollResponse { status: "expired".into() }).into_response();
    }

    let access_token = match token.access_token {
        Some(t) => t,
        None => return Json(PollResponse { status: "pending".into() }).into_response(),
    };

    // Fetch email and finish login — same as authorization_code callback
    #[derive(Deserialize)]
    struct UserInfo { email: String }

    let userinfo: UserInfo = match state
        .http_client
        .get("https://www.googleapis.com/oauth2/v2/userinfo")
        .bearer_auth(&access_token)
        .send()
        .await
    {
        Ok(r) => match r.json().await {
            Ok(u) => u,
            Err(e) => { warn!("device userinfo parse: {e}"); return StatusCode::BAD_GATEWAY.into_response(); }
        },
        Err(e) => { warn!("device userinfo request: {e}"); return StatusCode::BAD_GATEWAY.into_response(); }
    };

    let is_admin = {
        let cfg = state.config.read().unwrap();
        cfg.auth.admin_emails.iter().any(|e| e.eq_ignore_ascii_case(&userinfo.email))
    };
    if !is_admin {
        warn!("device login rejected for {}", userinfo.email);
        return Json(PollResponse { status: "expired".into() }).into_response();
    }

    let session_token = auth_util::generate_token();
    if let Err(e) = state.db.insert_session(&session_token) {
        warn!("insert session: {e}");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    let admin_profile_id: Option<i64> = match state.db.get_profile_by_email(&userinfo.email) {
        Ok(Some(p)) => Some(p.id),
        Ok(None) => match state.db.create_profile(&userinfo.email, Some(&userinfo.email), true) {
            Ok(p) => Some(p.id),
            Err(e) => { warn!("create admin profile: {e}"); None }
        },
        Err(e) => { warn!("get_profile_by_email: {e}"); None }
    };

    if let Some(profile_id) = admin_profile_id {
        let _ = state.db.set_session_profile(&session_token, profile_id);
    }

    // Remove the used device code entry
    state.device_codes.lock().unwrap().remove(&params.poll_token);

    let session_cookie = format!("session={}; Path=/; HttpOnly; SameSite=Lax; Max-Age=604800", session_token);
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(header::SET_COOKIE, session_cookie.parse().expect("valid cookie"));
    if let Some(profile_id) = admin_profile_id {
        let profile_cookie = format!("yt_plex_profile={profile_id}; Path=/; SameSite=Lax; Max-Age=604800");
        headers.append(header::SET_COOKIE, profile_cookie.parse().expect("valid cookie"));
    }

    (headers, Json(PollResponse { status: "done".into() })).into_response()
}

// ── Me ───────────────────────────────────────────────────────────────────────

pub async fn me(State(state): State<AppState>, SessionToken(token): SessionToken) -> StatusCode {
    match token {
        Some(t) if state.db.is_valid_session(&t).unwrap_or(false) => StatusCode::OK,
        _ => StatusCode::UNAUTHORIZED,
    }
}

// ── Admin profile ─────────────────────────────────────────────────────────────

pub async fn admin_profile(
    State(state): State<AppState>,
    SessionToken(token): SessionToken,
) -> impl IntoResponse {
    let Some(t) = token else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    match state.db.get_session_profile(&t) {
        Ok(Some(profile)) => Json(profile).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            warn!("get_session_profile: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
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
