use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub auth: AuthConfig,
    pub plex: PlexConfig,
    pub output: OutputConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub bind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub admin_password_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlexConfig {
    pub url: String,
    pub token: String,
    pub library_section_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    pub base_path: String,
    pub path_template: String,
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
admin_password_hash = "$argon2id$v=19$m=19456,t=2,p=1$abc$def"

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
        assert_eq!(config.plex.library_section_id, "1");
        assert_eq!(
            config.output.path_template,
            "{channel}/{date} - {title}.{ext}"
        );
    }
}
