use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
use tracing::error;

use crate::{routes::auth::SessionToken, sync, AppState};

fn is_admin(state: &AppState, token: Option<&str>) -> bool {
    token
        .and_then(|t| state.db.is_valid_session(t).ok())
        .unwrap_or(false)
}

pub async fn list_channels(State(state): State<AppState>) -> impl IntoResponse {
    match state.db.list_channels() {
        Ok(channels) => Json(channels).into_response(),
        Err(e) => {
            error!("list_channels: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Server error").into_response()
        }
    }
}

#[derive(Deserialize)]
pub struct AddChannelRequest {
    pub url: String,
    pub name: String,
}

pub async fn add_channel(
    State(state): State<AppState>,
    SessionToken(token): SessionToken,
    Json(body): Json<AddChannelRequest>,
) -> impl IntoResponse {
    if !is_admin(&state, token.as_deref()) {
        return (StatusCode::UNAUTHORIZED, "Admin required").into_response();
    }
    let channel = match state.db.insert_channel(&body.url, &body.name) {
        Ok(ch) => ch,
        Err(e) => {
            error!("insert_channel: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Server error").into_response();
        }
    };
    // Trigger first sync in background
    let db = Arc::clone(&state.db);
    let config = state.config.read().unwrap().clone();
    let ch_id = channel.id.clone();
    let ch_url = channel.youtube_channel_url.clone();
    tokio::spawn(async move {
        if let Err(e) = sync::sync_channel(&ch_id, &ch_url, &db, &config, true).await {
            error!("initial sync failed for {ch_url}: {e:#}");
        }
    });
    Json(channel).into_response()
}

pub async fn delete_channel(
    State(state): State<AppState>,
    SessionToken(token): SessionToken,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if !is_admin(&state, token.as_deref()) {
        return (StatusCode::UNAUTHORIZED, "Admin required").into_response();
    }
    match state.db.delete_channel(&id) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => {
            error!("delete_channel: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Server error").into_response()
        }
    }
}

pub async fn sync_channel(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let channel = match state.db.get_channel(&id) {
        Ok(Some(ch)) => ch,
        Ok(None) => return (StatusCode::NOT_FOUND, "Channel not found").into_response(),
        Err(e) => {
            error!("get_channel: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Server error").into_response();
        }
    };
    let db = Arc::clone(&state.db);
    let config = state.config.read().unwrap().clone();
    let ch_id = channel.id.clone();
    let ch_url = channel.youtube_channel_url.clone();
    let is_first = channel.last_synced_at.is_none();
    tokio::spawn(async move {
        if let Err(e) = sync::sync_channel(&ch_id, &ch_url, &db, &config, is_first).await {
            error!("manual sync failed for {ch_url}: {e:#}");
        }
    });
    StatusCode::ACCEPTED.into_response()
}

pub async fn list_channel_videos(
    State(state): State<AppState>,
    Path(id): Path<String>,
    axum::extract::Query(params): axum::extract::Query<VideoQueryParams>,
) -> impl IntoResponse {
    let filter = yt_plex_common::models::VideoFilter::from_str(
        params.filter.as_deref().unwrap_or("new"),
    );
    let show_ignored = params.show_ignored.unwrap_or(false);
    match state.db.list_videos_for_channel(&id, filter, show_ignored, None, 50, 0) {
        Ok(page) => Json(page).into_response(),
        Err(e) => {
            error!("list_videos_for_channel: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Server error").into_response()
        }
    }
}

#[derive(Deserialize)]
pub struct VideoQueryParams {
    pub filter: Option<String>,
    pub show_ignored: Option<bool>,
}
