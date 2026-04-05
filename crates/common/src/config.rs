use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub auth: AuthConfig,
    pub google_oauth: GoogleOAuthConfig,
    pub plex: PlexConfig,
    pub output: OutputConfig,
    #[serde(default)]
    pub sync: SyncConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub bind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub admin_emails: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleOAuthConfig {
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlexConfig {
    pub url: String,
    pub token: String,
    pub library_section_id: String,
}

fn default_thumbnail_cache_dir() -> String {
    let base = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
            PathBuf::from(home).join(".local").join("share")
        });
    base.join("yt-plex")
        .join("thumbnails")
        .to_string_lossy()
        .into_owned()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    pub base_path: String,
    pub path_template: String,
    #[serde(default = "default_thumbnail_cache_dir")]
    pub thumbnail_cache_dir: String,
}

fn default_interval_hours() -> u64 {
    6
}

fn default_playlist_items() -> usize {
    50
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    #[serde(default = "default_interval_hours")]
    pub interval_hours: u64,
    #[serde(default = "default_playlist_items")]
    pub playlist_items: usize,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            interval_hours: default_interval_hours(),
            playlist_items: default_playlist_items(),
        }
    }
}

pub fn default_config_path() -> String {
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
            PathBuf::from(home).join(".config")
        });
    base.join("yt-plex")
        .join("config.toml")
        .to_string_lossy()
        .into_owned()
}

pub fn load_config(path: &str) -> Result<Config> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("reading config: {path}"))?;
    toml::from_str(&text).context("parsing config")
}

pub fn save_config(path: &str, config: &Config) -> Result<()> {
    let text = toml::to_string_pretty(config).context("serialising config")?;
    if let Some(parent) = std::path::Path::new(path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, text).with_context(|| format!("writing config: {path}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_config() {
        let toml = r#"
[server]
bind = "127.0.0.1:3000"

[auth]
admin_emails = ["admin@example.com"]

[google_oauth]
client_id = "fake_client_id"
client_secret = "fake_secret"

[plex]
url = "http://localhost:32400"
token = "mytoken"
library_section_id = "1"

[output]
base_path = "/mnt/plex"
path_template = "{channel}/{date} - {title}.{ext}"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.server.bind, "127.0.0.1:3000");
        assert_eq!(config.auth.admin_emails, vec!["admin@example.com"]);
        assert_eq!(config.google_oauth.client_id, "fake_client_id");
        assert_eq!(config.plex.library_section_id, "1");
        assert_eq!(
            config.output.path_template,
            "{channel}/{date} - {title}.{ext}"
        );
    }

    #[test]
    fn parses_config_with_sync_section() {
        let toml = r#"
[server]
bind = "127.0.0.1:3000"
[auth]
admin_emails = ["admin@example.com"]
[google_oauth]
client_id = "cid"
client_secret = "csec"
[plex]
url = "http://localhost:32400"
token = "tok"
library_section_id = "1"
[output]
base_path = "/mnt/plex"
path_template = "{channel}/{date} - {title} [{id}].{ext}"
[sync]
interval_hours = 12
playlist_items = 100
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.sync.interval_hours, 12);
        assert_eq!(config.sync.playlist_items, 100);
    }

    #[test]
    fn sync_config_defaults_when_section_missing() {
        let toml = r#"
[server]
bind = "127.0.0.1:3000"
[auth]
admin_emails = []
[google_oauth]
client_id = "cid"
client_secret = "csec"
[plex]
url = "http://localhost:32400"
token = "tok"
library_section_id = "1"
[output]
base_path = "/mnt/plex"
path_template = "{channel}/{date} - {title}.{ext}"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.sync.interval_hours, 6);
        assert_eq!(config.sync.playlist_items, 50);
    }
}
