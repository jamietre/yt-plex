use anyhow::Result;
use serde::Serialize;
use yt_plex_common::config::PlexConfig;

#[derive(Serialize)]
pub struct PlexLibrary {
    pub id: String,
    pub title: String,
    pub lib_type: String,
}

pub async fn list_libraries(config: &PlexConfig) -> Result<Vec<PlexLibrary>> {
    let url = format!("{}/library/sections", config.url.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("X-Plex-Token", &config.token)
        .header("Accept", "application/json")
        .send()
        .await?;
    if !resp.status().is_success() {
        anyhow::bail!("Plex returned {}", resp.status());
    }
    let json: serde_json::Value = resp.json().await?;
    let dirs = json["MediaContainer"]["Directory"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("unexpected Plex response format"))?;
    Ok(dirs
        .iter()
        .filter_map(|d| {
            Some(PlexLibrary {
                id: d["key"].as_str()?.to_string(),
                title: d["title"].as_str()?.to_string(),
                lib_type: d["type"].as_str().unwrap_or("").to_string(),
            })
        })
        .collect())
}

pub async fn trigger_library_refresh(config: &PlexConfig) -> Result<()> {
    let client = reqwest::Client::new();
    let base = config.url.trim_end_matches('/');
    for id in config.library_section_id.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let url = format!("{base}/library/sections/{id}/refresh");
        let resp = client
            .get(&url)
            .header("X-Plex-Token", &config.token)
            .send()
            .await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Plex refresh failed for section {id} {status}: {body}");
        }
    }
    Ok(())
}
