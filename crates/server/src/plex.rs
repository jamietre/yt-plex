use anyhow::Result;
use yt_plex_common::config::PlexConfig;

pub async fn trigger_library_refresh(config: &PlexConfig) -> Result<()> {
    let url = format!(
        "{}/library/sections/{}/refresh",
        config.url.trim_end_matches('/'),
        config.library_section_id
    );
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("X-Plex-Token", &config.token)
        .send()
        .await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Plex refresh failed {status}: {body}");
    }
    Ok(())
}
