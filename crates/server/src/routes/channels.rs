use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::{routes::auth::SessionToken, routes::profiles::ProfileCookie, sync, AppState};

fn is_admin(state: &AppState, token: Option<&str>) -> bool {
    token
        .and_then(|t| state.db.is_valid_session(t).ok())
        .unwrap_or(false)
}

#[derive(Deserialize)]
pub struct ListChannelsParams {
    /// When true, return all channels regardless of profile subscriptions.
    /// Useful for the subscription management UI.
    pub all: Option<bool>,
}

pub async fn list_channels(
    State(state): State<AppState>,
    SessionToken(token): SessionToken,
    ProfileCookie(profile_id): ProfileCookie,
    axum::extract::Query(params): axum::extract::Query<ListChannelsParams>,
) -> impl IntoResponse {
    // Admin always sees all; ?all=true bypasses profile filter for non-admin too
    let show_all = is_admin(&state, token.as_deref()) || params.all.unwrap_or(false);
    let effective_profile = if show_all { None } else { profile_id };
    match state.db.list_channels_for_profile(effective_profile) {
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
    SessionToken(token): SessionToken,
    ProfileCookie(profile_id): ProfileCookie,
    Path(id): Path<String>,
    axum::extract::Query(params): axum::extract::Query<VideoQueryParams>,
) -> impl IntoResponse {
    let filter = yt_plex_common::models::VideoFilter::parse(
        params.filter.as_deref().unwrap_or("new"),
    );
    let show_ignored = params.show_ignored.unwrap_or(false);
    let search = params.q.as_deref().filter(|s| !s.is_empty());
    let limit = params.limit.unwrap_or(48).min(200);
    let offset = params.offset.unwrap_or(0);
    // Admin ignores are global (None); user profiles use per-profile ignores
    let effective_profile = if is_admin(&state, token.as_deref()) { None } else { profile_id };
    match state.db.list_videos_for_channel(&id, filter, show_ignored, search, limit, offset, effective_profile) {
        Ok(page) => Json(page).into_response(),
        Err(e) => {
            error!("list_videos_for_channel: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Server error").into_response()
        }
    }
}

/// Trigger a filesystem re-scan: marks present videos as downloaded, clears
/// stale downloaded status for files no longer on disk.
pub async fn rescan_filesystem(
    State(state): State<AppState>,
    SessionToken(token): SessionToken,
) -> impl IntoResponse {
    if !is_admin(&state, token.as_deref()) {
        return (StatusCode::UNAUTHORIZED, "Admin required").into_response();
    }
    let base_path = state.config.read().unwrap().output.base_path.clone();
    let db = Arc::clone(&state.db);
    tokio::spawn(async move {
        if let Err(e) = tokio::task::spawn_blocking(move || sync::scan_filesystem(&base_path, &db)).await {
            error!("rescan_filesystem task: {e:#}");
        }
    });
    StatusCode::ACCEPTED.into_response()
}

#[derive(Serialize)]
pub struct RegenMetaResponse {
    pub queued: usize,
}

/// POST /api/channels/{id}/regen-metadata
/// Re-runs yt-dlp --skip-download --write-info-json for every downloaded video
/// in the channel, regenerating .info.json sidecars without redownloading.
pub async fn regen_metadata(
    State(state): State<AppState>,
    SessionToken(token): SessionToken,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if !is_admin(&state, token.as_deref()) {
        return (StatusCode::UNAUTHORIZED, "Admin required").into_response();
    }

    let videos = match state.db.list_downloaded_videos_for_channel(&id) {
        Ok(v) => v,
        Err(e) => {
            error!("list_downloaded_videos_for_channel: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Server error").into_response();
        }
    };

    let count = videos.len();
    let extra_args = state.config.read().unwrap().download.extra_args.clone();

    tokio::spawn(async move {
        for (youtube_id, file_path) in videos {
            // Derive the output template from the existing file path so yt-dlp
            // writes <name>.info.json alongside the video file.
            let stem = std::path::Path::new(&file_path)
                .with_extension("")
                .to_string_lossy()
                .into_owned();
            let out_template = format!("{}.%(ext)s", stem);
            let url = format!("https://www.youtube.com/watch?v={}", youtube_id);

            info!("regen metadata: {youtube_id}");
            let status = tokio::process::Command::new("yt-dlp")
                .args(["--skip-download", "--write-info-json", "--no-clean-info-json", "-o", &out_template])
                .args(&extra_args)
                .arg(&url)
                .status()
                .await;

            match status {
                Ok(s) if s.success() => info!("regen ok: {youtube_id}"),
                Ok(s) => warn!("regen failed for {youtube_id}: exit {s}"),
                Err(e) => warn!("regen error for {youtube_id}: {e}"),
            }
        }
        info!("regen metadata complete");
    });

    Json(RegenMetaResponse { queued: count }).into_response()
}

#[derive(Deserialize)]
pub struct VideoQueryParams {
    pub filter: Option<String>,
    pub show_ignored: Option<bool>,
    pub q: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}
