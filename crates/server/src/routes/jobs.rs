use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;

use crate::{routes::auth::SessionToken, AppState};

fn is_admin(state: &AppState, token: Option<&str>) -> bool {
    token
        .and_then(|t| state.db.is_valid_session(t).ok())
        .unwrap_or(false)
}

#[derive(Deserialize)]
pub struct SubmitJobRequest {
    pub url: String,
}

pub async fn submit_job(
    State(state): State<AppState>,
    SessionToken(token): SessionToken,
    Json(body): Json<SubmitJobRequest>,
) -> impl IntoResponse {
    if !is_admin(&state, token.as_deref()) {
        return (StatusCode::UNAUTHORIZED, "Admin required").into_response();
    }
    if !body.url.starts_with("https://www.youtube.com/")
        && !body.url.starts_with("https://youtu.be/")
    {
        return (StatusCode::BAD_REQUEST, "Only YouTube URLs are accepted").into_response();
    }
    match state.db.insert_job(&body.url) {
        Ok(job) => Json(job).into_response(),
        Err(e) => {
            tracing::error!("insert job: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Server error").into_response()
        }
    }
}

pub async fn list_jobs(State(state): State<AppState>) -> impl IntoResponse {
    match state.db.list_jobs() {
        Ok(jobs) => Json(jobs).into_response(),
        Err(e) => {
            tracing::error!("list jobs: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Server error").into_response()
        }
    }
}
