use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::IntoResponse,
};
use std::path::PathBuf;
use tracing::{error, warn};

use crate::AppState;

/// Serve a YouTube thumbnail, caching it locally on first request.
///
/// Tries `mqdefault.jpg` (320×180) first; falls back to `hqdefault.jpg`
/// if YouTube returns 404 (some older/private videos lack mqdefault).
pub async fn get_thumbnail(
    State(state): State<AppState>,
    Path(youtube_id): Path<String>,
) -> impl IntoResponse {
    // Sanitise: youtube IDs are alphanumeric + - _
    if !youtube_id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return (StatusCode::BAD_REQUEST, "invalid id").into_response();
    }

    let cache_dir = state.config.read().unwrap().output.thumbnail_cache_dir.clone();
    let cache_path = PathBuf::from(&cache_dir).join(format!("{youtube_id}.jpg"));

    // Serve from cache if available.
    if let Ok(bytes) = tokio::fs::read(&cache_path).await {
        return jpeg_response(axum::body::Bytes::from(bytes));
    }

    // Fetch from YouTube.
    let bytes = match fetch_thumbnail(&state.http_client, &youtube_id).await {
        Ok(b) => b,
        Err(e) => {
            warn!("thumbnail fetch for {youtube_id}: {e:#}");
            return StatusCode::NOT_FOUND.into_response();
        }
    };

    // Persist to cache (best-effort; don't fail the request if disk write fails).
    if let Err(e) = persist(&cache_path, &bytes).await {
        error!("thumbnail cache write for {youtube_id}: {e:#}");
    }

    jpeg_response(bytes)
}

async fn fetch_thumbnail(
    client: &reqwest::Client,
    youtube_id: &str,
) -> anyhow::Result<axum::body::Bytes> {
    // Try medium quality first (always available for public videos), then high quality fallback.
    for quality in &["mqdefault", "hqdefault"] {
        let url = format!("https://img.youtube.com/vi/{youtube_id}/{quality}.jpg");
        let resp = client.get(&url).send().await?;
        if resp.status().is_success() {
            return Ok(resp.bytes().await?);
        }
    }
    anyhow::bail!("no thumbnail available for {youtube_id}")
}

async fn persist(path: &PathBuf, bytes: &axum::body::Bytes) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(path, bytes).await?;
    Ok(())
}

fn jpeg_response(bytes: axum::body::Bytes) -> axum::response::Response {
    (
        [
            (header::CONTENT_TYPE, "image/jpeg"),
            (header::CACHE_CONTROL, "public, max-age=604800"), // 1 week
        ],
        bytes,
    )
        .into_response()
}
