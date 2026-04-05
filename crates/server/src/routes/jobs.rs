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
    pub url: Option<String>,
    pub youtube_id: Option<String>,
}

pub async fn submit_job(
    State(state): State<AppState>,
    SessionToken(token): SessionToken,
    Json(body): Json<SubmitJobRequest>,
) -> impl IntoResponse {
    // (url, channel_name, title)
    let (url, prefill_channel, prefill_title) = if let Some(youtube_id) = &body.youtube_id {
        // Any user may queue a download by youtube_id — but only if the video
        // exists in an approved channel.
        match state.db.get_video(youtube_id) {
            Ok(Some(video)) => {
                let channel_name = state.db.get_channel(&video.channel_id)
                    .ok().flatten().map(|c| c.name);
                let url = format!("https://www.youtube.com/watch?v={youtube_id}");
                (url, channel_name, Some(video.title))
            }
            Ok(None) => {
                return (StatusCode::NOT_FOUND, "Video not in any approved channel").into_response()
            }
            Err(e) => {
                tracing::error!("get_video: {e}");
                return (StatusCode::INTERNAL_SERVER_ERROR, "Server error").into_response();
            }
        }
    } else if let Some(url) = &body.url {
        // Arbitrary URL submission — admin only.
        if !is_admin(&state, token.as_deref()) {
            return (StatusCode::UNAUTHORIZED, "Admin required").into_response();
        }
        if !url.starts_with("https://www.youtube.com/")
            && !url.starts_with("https://youtu.be/")
        {
            return (StatusCode::BAD_REQUEST, "Only YouTube URLs are accepted").into_response();
        }
        (url.clone(), None, None)
    } else {
        return (StatusCode::BAD_REQUEST, "Provide either url or youtube_id").into_response();
    };

    match state.db.insert_job(&url, prefill_channel.as_deref(), prefill_title.as_deref()) {
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
