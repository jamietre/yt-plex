use axum::{extract::{Path, State}, http::StatusCode, response::IntoResponse};
use chrono::Utc;
use tracing::error;
use crate::AppState;

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
