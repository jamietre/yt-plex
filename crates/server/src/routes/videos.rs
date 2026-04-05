use axum::{extract::{Path, State}, http::StatusCode, response::IntoResponse, Json};
use chrono::Utc;
use tracing::error;
use crate::{sync, AppState};

pub async fn get_video(
    State(state): State<AppState>,
    Path(youtube_id): Path<String>,
) -> impl IntoResponse {
    // Check video exists in DB
    let video = match state.db.get_video(&youtube_id) {
        Ok(Some(v)) => v,
        Ok(None) => return (StatusCode::NOT_FOUND, "Video not found").into_response(),
        Err(e) => {
            error!("get_video: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Server error").into_response();
        }
    };
    // If description not yet cached, fetch it now (synchronous, ~3s)
    if video.description.is_none() {
        match sync::fetch_video_description(&youtube_id).await {
            Ok(desc) => {
                if let Err(e) = state.db.set_video_description(&youtube_id, &desc) {
                    error!("set_video_description: {e}");
                }
                // Re-fetch to get updated description
                return match state.db.get_video(&youtube_id) {
                    Ok(Some(v)) => Json(v).into_response(),
                    Ok(None) => (StatusCode::NOT_FOUND, "Video not found").into_response(),
                    Err(e) => {
                        error!("get_video after desc update: {e}");
                        (StatusCode::INTERNAL_SERVER_ERROR, "Server error").into_response()
                    }
                };
            }
            Err(e) => {
                // Non-fatal: return the video without description
                error!("fetch_video_description for {youtube_id}: {e:#}");
            }
        }
    }
    Json(video).into_response()
}

pub async fn ignore_video(
    State(state): State<AppState>,
    Path(youtube_id): Path<String>,
) -> impl IntoResponse {
    let now = Utc::now().to_rfc3339();
    match state.db.ignore_video(&youtube_id, &now) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => {
            error!("ignore_video: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Server error").into_response()
        }
    }
}

pub async fn unignore_video(
    State(state): State<AppState>,
    Path(youtube_id): Path<String>,
) -> impl IntoResponse {
    match state.db.unignore_video(&youtube_id) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => {
            error!("unignore_video: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Server error").into_response()
        }
    }
}
