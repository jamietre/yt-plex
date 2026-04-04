pub mod auth;
pub mod db;
pub mod plex;
pub mod routes;
pub mod template;
pub mod worker;
pub mod ws;

use std::sync::Arc;
use yt_plex_common::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<crate::db::Db>,
    pub config: Arc<std::sync::RwLock<Config>>,
    pub config_path: String,
    pub ws_hub: crate::ws::WsHub,
}
