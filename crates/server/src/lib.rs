pub mod auth;
pub mod db;
pub mod plex;
pub mod routes;
pub mod sync;
pub mod template;
pub mod worker;
pub mod ws;

use anyhow::{bail, Result};
use axum::{
    routing::{delete, get, post, put},
    Router,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// State stored server-side for a pending device-flow login.
#[derive(Clone)]
pub struct DeviceCodeEntry {
    pub device_code: String,
    pub expires_at: Instant,
}

/// CSRF state entry for the authorization_code flow.
#[derive(Clone)]
pub struct OAuthStateEntry {
    pub expires_at: Instant,
    /// Origin the login was initiated from (e.g. "http://PRIVATE_IP_REDACTED:32113").
    /// Used to redirect back to the local app after the public callback completes.
    pub return_to: Option<String>,
}

/// Short-lived token bridging the public callback to the local app session cookie.
#[derive(Clone)]
pub struct ExchangeTokenEntry {
    pub session_token: String,
    pub profile_id: Option<i64>,
    pub expires_at: Instant,
}
use tower_http::trace::TraceLayer;
use yt_plex_common::config::Config;

use crate::{db::Db, ws::WsHub};

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Db>,
    pub config: Arc<std::sync::RwLock<Config>>,
    pub config_path: String,
    pub ws_hub: WsHub,
    /// CSRF state tokens (authorization_code flow).
    pub oauth_states: Arc<Mutex<HashMap<String, OAuthStateEntry>>>,
    /// Pending device-flow logins: poll_token → DeviceCodeEntry.
    pub device_codes: Arc<Mutex<HashMap<String, DeviceCodeEntry>>>,
    /// Short-lived tokens bridging the public callback to the local app.
    pub exchange_tokens: Arc<Mutex<HashMap<String, ExchangeTokenEntry>>>,
    pub http_client: reqwest::Client,
}

pub async fn create_app_state(config: Config, config_path: String) -> Result<AppState> {
    let db_path = {
        let base = std::env::var("XDG_DATA_HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
                std::path::PathBuf::from(home)
                    .join(".local")
                    .join("share")
            });
        base.join("yt-plex").join("db.sqlite")
    };
    if let Some(p) = db_path.parent() {
        std::fs::create_dir_all(p)?;
    }
    let db = Arc::new(Db::open(db_path.to_str().unwrap())?);
    let reset = db.reset_interrupted_jobs()?;
    if reset > 0 {
        tracing::warn!("reset {reset} interrupted job(s) back to queued");
    }

    // Check required external dependencies
    check_dependencies()?;

    let ws_hub = WsHub::new();
    let config_arc = Arc::new(std::sync::RwLock::new(config));

    // Spawn background channel sync loop
    {
        let sync_db = Arc::clone(&db);
        let sync_config = Arc::clone(&config_arc);
        tokio::spawn(async move {
            crate::sync::run_sync_loop(sync_db, sync_config).await;
        });
    }

    Ok(AppState {
        db,
        config: config_arc,
        config_path,
        ws_hub,
        oauth_states: Arc::new(Mutex::new(HashMap::new())),
        device_codes: Arc::new(Mutex::new(HashMap::new())),
        exchange_tokens: Arc::new(Mutex::new(HashMap::new())),
        http_client: reqwest::Client::new(),
    })
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/api/auth/flow", get(routes::auth::auth_flow))
        .route("/api/auth/login", get(routes::auth::oauth_login))
        .route("/api/auth/callback", get(routes::auth::oauth_callback))
        .route("/api/auth/exchange", get(routes::auth::exchange))
        .route("/api/auth/device", post(routes::auth::device_start))
        .route("/api/auth/poll", post(routes::auth::device_poll))
        .route("/api/auth/me", get(routes::auth::me))
        .route("/api/auth/admin-profile", get(routes::auth::admin_profile))
        .route("/api/logout", post(routes::auth::logout))
        .route("/api/jobs", post(routes::jobs::submit_job))
        .route("/api/jobs", get(routes::jobs::list_jobs))
        .route("/api/settings", get(routes::settings::get_settings))
        .route("/api/settings", put(routes::settings::update_settings))
        .route("/api/plex/libraries", get(routes::settings::list_plex_libraries))

        .route("/api/channels", get(routes::channels::list_channels))
        .route("/api/channels", post(routes::channels::add_channel))
        .route("/api/channels/{id}", delete(routes::channels::delete_channel))
        .route("/api/channels/{id}", put(routes::channels::update_channel))
        .route("/api/channels/{id}/sync", post(routes::channels::sync_channel))
        .route("/api/channels/{id}/regen-metadata", post(routes::channels::regen_metadata))
        .route("/api/channels/{id}/videos", get(routes::channels::list_channel_videos))
        .route("/api/rescan", post(routes::channels::rescan_filesystem))
        .route("/api/videos/{youtube_id}", get(routes::videos::get_video))
        .route("/api/videos/{youtube_id}/ignore", post(routes::videos::ignore_video))
        .route("/api/videos/{youtube_id}/ignore", delete(routes::videos::unignore_video))
        .route("/api/thumbnails/{youtube_id}", get(routes::thumbnails::get_thumbnail))
        .route("/api/profiles", get(routes::profiles::list_profiles))
        .route("/api/profiles", post(routes::profiles::create_profile))
        .route("/api/profiles/{id}", delete(routes::profiles::delete_profile))
        .route("/api/profile-session", get(routes::profiles::get_session))
        .route("/api/profile-session", post(routes::profiles::set_session))
        .route("/api/profile-session", delete(routes::profiles::clear_session))
        .route("/api/profiles/{id}/channels", get(routes::profiles::list_profile_channels))
        .route("/api/profiles/{id}/channels/{cid}", put(routes::profiles::subscribe_channel))
        .route("/api/profiles/{id}/channels/{cid}", delete(routes::profiles::unsubscribe_channel))
        .route("/ws", get(ws::ws_handler))
        .fallback(routes::assets::serve_asset)
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}

fn check_dependencies() -> Result<()> {
    let status = std::process::Command::new("yt-dlp")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    match status {
        Ok(s) if s.success() => Ok(()),
        _ => bail!("yt-dlp not found on PATH — install it before starting the server"),
    }
}
