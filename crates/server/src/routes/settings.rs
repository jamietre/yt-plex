use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use tracing::error;
use serde::Deserialize;
use yt_plex_common::config::{DownloadConfig, OutputConfig, PlexConfig};

use crate::{routes::auth::SessionToken, AppState};

fn is_admin(state: &AppState, token: Option<&str>) -> bool {
    token
        .and_then(|t| state.db.is_valid_session(t).ok())
        .unwrap_or(false)
}

pub async fn get_settings(
    State(state): State<AppState>,
    SessionToken(token): SessionToken,
) -> impl IntoResponse {
    if !is_admin(&state, token.as_deref()) {
        return (StatusCode::UNAUTHORIZED, "Admin required").into_response();
    }
    let cfg = state.config.read().unwrap();
    Json(serde_json::json!({
        "plex": cfg.plex,
        "output": cfg.output,
        "download": cfg.download,
    }))
    .into_response()
}

#[derive(Deserialize)]
pub struct UpdateSettingsRequest {
    pub plex: PlexConfig,
    pub output: OutputConfig,
    #[serde(default)]
    pub download: DownloadConfig,
}

pub async fn update_settings(
    State(state): State<AppState>,
    SessionToken(token): SessionToken,
    Json(body): Json<UpdateSettingsRequest>,
) -> impl IntoResponse {
    if !is_admin(&state, token.as_deref()) {
        return (StatusCode::UNAUTHORIZED, "Admin required").into_response();
    }
    {
        let mut cfg = state.config.write().unwrap();
        cfg.plex = body.plex;
        cfg.output = body.output;
        cfg.download = body.download;
    }
    let cfg = state.config.read().unwrap().clone();
    let path = state.config_path.clone();
    if let Err(e) = yt_plex_common::config::save_config(&path, &cfg) {
        tracing::error!("saving config: {e}");
        return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to save config").into_response();
    }
    StatusCode::OK.into_response()
}

pub async fn list_plex_libraries(
    State(state): State<AppState>,
    SessionToken(token): SessionToken,
) -> impl IntoResponse {
    if !is_admin(&state, token.as_deref()) {
        return (StatusCode::UNAUTHORIZED, "Admin required").into_response();
    }
    let config = state.config.read().unwrap().plex.clone();
    match crate::plex::list_libraries(&config).await {
        Ok(libs) => Json(libs).into_response(),
        Err(e) => {
            error!("list_plex_libraries: {e}");
            (StatusCode::BAD_GATEWAY, e.to_string()).into_response()
        }
    }
}
