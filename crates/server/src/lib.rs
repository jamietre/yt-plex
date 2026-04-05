pub mod auth;
pub mod db;
pub mod plex;
pub mod routes;
pub mod template;
pub mod worker;
pub mod ws;

use anyhow::{bail, Result};
use axum::{
    routing::{get, post, put},
    Router,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tower_http::trace::TraceLayer;
use yt_plex_common::config::Config;

use crate::{db::Db, ws::WsHub};

pub struct DeviceCodeEntry {
    pub google_device_code: String,
    pub expires_at: Instant,
    pub interval: u64,
}

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Db>,
    pub config: Arc<std::sync::RwLock<Config>>,
    pub config_path: String,
    pub ws_hub: WsHub,
    pub oauth_states: Arc<Mutex<HashMap<String, DeviceCodeEntry>>>,
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
    Ok(AppState {
        db,
        config: Arc::new(std::sync::RwLock::new(config)),
        config_path,
        ws_hub,
        oauth_states: Arc::new(Mutex::new(HashMap::new())),
        http_client: reqwest::Client::new(),
    })
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/api/auth/login", get(routes::auth::device_login))
        .route("/api/auth/poll", get(routes::auth::device_poll))
        .route("/api/auth/me", get(routes::auth::me))
        .route("/api/logout", post(routes::auth::logout))
        .route("/api/jobs", post(routes::jobs::submit_job))
        .route("/api/jobs", get(routes::jobs::list_jobs))
        .route("/api/settings", get(routes::settings::get_settings))
        .route("/api/settings", put(routes::settings::update_settings))
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
