use axum::{
    extract::{FromRequestParts, Path, State},
    http::{header, request::Parts, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use tracing::error;

use crate::{routes::auth::SessionToken, AppState};

// ── Profile cookie extractor ──────────────────────────────────────────────────

/// Extracts the `yt_plex_profile` cookie value as `Option<i64>`.
pub struct ProfileCookie(pub Option<i64>);

impl<S> FromRequestParts<S> for ProfileCookie
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let id = parts
            .headers
            .get(header::COOKIE)
            .and_then(|v| v.to_str().ok())
            .and_then(|cookies| {
                cookies
                    .split(';')
                    .map(|c| c.trim())
                    .find(|c| c.starts_with("yt_plex_profile="))
                    .and_then(|c| c["yt_plex_profile=".len()..].parse::<i64>().ok())
            });
        Ok(ProfileCookie(id))
    }
}

fn profile_cookie_header(profile_id: i64) -> String {
    format!("yt_plex_profile={profile_id}; Path=/; SameSite=Lax; Max-Age=604800")
}

fn clear_profile_cookie() -> &'static str {
    "yt_plex_profile=; Path=/; SameSite=Lax; Max-Age=0"
}

fn is_admin(state: &AppState, token: Option<&str>) -> bool {
    token
        .and_then(|t| state.db.is_valid_session(t).ok())
        .unwrap_or(false)
}

// ── Profile CRUD ──────────────────────────────────────────────────────────────

pub async fn list_profiles(State(state): State<AppState>) -> impl IntoResponse {
    match state.db.list_profiles(false) {
        Ok(profiles) => Json(profiles).into_response(),
        Err(e) => {
            error!("list_profiles: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Server error").into_response()
        }
    }
}

#[derive(Deserialize)]
pub struct CreateProfileRequest {
    pub name: String,
}

pub async fn create_profile(
    State(state): State<AppState>,
    SessionToken(token): SessionToken,
    Json(body): Json<CreateProfileRequest>,
) -> impl IntoResponse {
    if !is_admin(&state, token.as_deref()) {
        return (StatusCode::UNAUTHORIZED, "Admin required").into_response();
    }
    match state.db.create_profile(&body.name, None, false) {
        Ok(profile) => (StatusCode::CREATED, Json(profile)).into_response(),
        Err(e) => {
            error!("create_profile: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Server error").into_response()
        }
    }
}

pub async fn delete_profile(
    State(state): State<AppState>,
    SessionToken(token): SessionToken,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    if !is_admin(&state, token.as_deref()) {
        return (StatusCode::UNAUTHORIZED, "Admin required").into_response();
    }
    // Refuse to delete admin profiles via this endpoint
    match state.db.get_profile(id) {
        Ok(Some(p)) if p.is_admin_profile => {
            return (StatusCode::FORBIDDEN, "Cannot delete admin profile").into_response();
        }
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            error!("get_profile: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Server error").into_response();
        }
        Ok(Some(_)) => {}
    }
    match state.db.delete_profile(id) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => {
            error!("delete_profile: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Server error").into_response()
        }
    }
}

// ── Profile session cookie ────────────────────────────────────────────────────

pub async fn get_session(
    State(state): State<AppState>,
    ProfileCookie(profile_id): ProfileCookie,
) -> impl IntoResponse {
    let Some(id) = profile_id else {
        return (StatusCode::NO_CONTENT, "").into_response();
    };
    match state.db.get_profile(id) {
        Ok(Some(profile)) => Json(profile).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "Profile not found").into_response(),
        Err(e) => {
            error!("get_profile: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Server error").into_response()
        }
    }
}

#[derive(Deserialize)]
pub struct SetSessionRequest {
    pub profile_id: i64,
}

pub async fn set_session(
    State(state): State<AppState>,
    Json(body): Json<SetSessionRequest>,
) -> impl IntoResponse {
    match state.db.get_profile(body.profile_id) {
        Ok(Some(_)) => {}
        Ok(None) => return (StatusCode::NOT_FOUND, "Profile not found").into_response(),
        Err(e) => {
            error!("get_profile: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Server error").into_response();
        }
    }
    let cookie = profile_cookie_header(body.profile_id);
    (StatusCode::OK, [(header::SET_COOKIE, cookie)], "").into_response()
}

pub async fn clear_session() -> impl IntoResponse {
    (
        StatusCode::OK,
        [(header::SET_COOKIE, clear_profile_cookie())],
        "",
    )
}

// ── Channel subscriptions ─────────────────────────────────────────────────────

/// Check if the caller is authorized to modify a profile's subscriptions.
/// Allowed when: caller is admin, or the caller's profile cookie matches `profile_id`.
fn can_modify_profile(
    state: &AppState,
    token: Option<&str>,
    caller_profile: Option<i64>,
    profile_id: i64,
) -> bool {
    is_admin(state, token) || caller_profile == Some(profile_id)
}

pub async fn list_profile_channels(
    State(state): State<AppState>,
    SessionToken(token): SessionToken,
    ProfileCookie(caller_profile): ProfileCookie,
    Path(profile_id): Path<i64>,
) -> impl IntoResponse {
    if !can_modify_profile(&state, token.as_deref(), caller_profile, profile_id) {
        return (StatusCode::UNAUTHORIZED, "Not authorized").into_response();
    }
    match state.db.list_profile_channel_ids(profile_id) {
        Ok(ids) => Json(ids).into_response(),
        Err(e) => {
            error!("list_profile_channel_ids: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Server error").into_response()
        }
    }
}

pub async fn subscribe_channel(
    State(state): State<AppState>,
    SessionToken(token): SessionToken,
    ProfileCookie(caller_profile): ProfileCookie,
    Path((profile_id, channel_id)): Path<(i64, String)>,
) -> impl IntoResponse {
    if !can_modify_profile(&state, token.as_deref(), caller_profile, profile_id) {
        return (StatusCode::UNAUTHORIZED, "Not authorized").into_response();
    }
    match state.db.subscribe_channel(profile_id, &channel_id) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => {
            error!("subscribe_channel: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Server error").into_response()
        }
    }
}

pub async fn unsubscribe_channel(
    State(state): State<AppState>,
    SessionToken(token): SessionToken,
    ProfileCookie(caller_profile): ProfileCookie,
    Path((profile_id, channel_id)): Path<(i64, String)>,
) -> impl IntoResponse {
    if !can_modify_profile(&state, token.as_deref(), caller_profile, profile_id) {
        return (StatusCode::UNAUTHORIZED, "Not authorized").into_response();
    }
    match state.db.unsubscribe_channel(profile_id, &channel_id) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => {
            error!("unsubscribe_channel: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Server error").into_response()
        }
    }
}
