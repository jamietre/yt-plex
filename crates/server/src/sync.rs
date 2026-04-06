use anyhow::{Context, Result};
use chrono::Utc;
use serde::Deserialize;
use std::path::Path;
use std::time::Duration;
use tokio::io::AsyncBufReadExt;
use tokio::process::Command;
use tracing::{error, info, warn};
use yt_plex_common::config::Config;

use crate::db::Db;

#[derive(Debug, PartialEq)]
pub struct FlatPlaylistEntry {
    pub youtube_id: String,
    pub title: String,
    pub published_at: Option<String>,
    /// YouTube channel ID (UCxxxxxxxxxxxxxxxx). Same for every entry in a channel playlist.
    pub channel_id: Option<String>,
}

/// Parse one line of yt-dlp flat-playlist output.
/// Format: `%(id)s\t%(title)s\t%(upload_date)s\t%(channel_id)s`
pub fn parse_flat_playlist_line(line: &str) -> Option<FlatPlaylistEntry> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    let mut parts = line.splitn(4, '\t');
    let youtube_id = parts.next()?.trim().to_string();
    if youtube_id.is_empty() {
        return None;
    }
    let title = parts.next().unwrap_or("").trim().to_string();
    let date_raw = parts.next().unwrap_or("").trim();
    let published_at = parse_upload_date(date_raw);
    let channel_id_raw = parts.next().unwrap_or("").trim().to_string();
    // yt-dlp prints "NA" when a field is unavailable
    let channel_id = if channel_id_raw.is_empty() || channel_id_raw == "NA" {
        None
    } else {
        Some(channel_id_raw)
    };
    Some(FlatPlaylistEntry { youtube_id, title, published_at, channel_id })
}

pub fn parse_upload_date(s: &str) -> Option<String> {
    if s.len() == 8 && s.chars().all(|c| c.is_ascii_digit()) {
        Some(format!("{}-{}-{}", &s[0..4], &s[4..6], &s[6..8]))
    } else {
        None
    }
}

/// Extract a YouTube video ID from `[youtube_id]` suffix in a filename stem.
/// Uses the last `[...]` bracket pair found.
pub fn extract_youtube_id_from_path(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?;
    let open = stem.rfind('[')?;
    let close = stem.rfind(']')?;
    if close > open + 1 {
        Some(stem[open + 1..close].to_string())
    } else {
        None
    }
}

#[derive(Deserialize)]
struct YtDlpVideoMeta {
    description: Option<String>,
    /// Upload date in YYYYMMDD format, as returned by yt-dlp.
    upload_date: Option<String>,
}

pub struct VideoMeta {
    pub description: String,
    /// Upload date formatted as YYYY-MM-DD, or None if unavailable.
    pub published_at: Option<String>,
}

/// Fetch full metadata for a single video via yt-dlp -j.
/// Returns description and the real upload date (reliable, unlike flat-playlist).
/// Takes ~2–5 seconds per video.
pub async fn fetch_video_meta(youtube_id: &str) -> Result<VideoMeta> {
    let url = format!("https://www.youtube.com/watch?v={youtube_id}");
    let output = Command::new("yt-dlp")
        .args(["--no-playlist", "-j", &url])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .await
        .context("spawning yt-dlp for video meta fetch")?;

    if !output.status.success() {
        anyhow::bail!("yt-dlp exited with status {}", output.status);
    }
    let meta: YtDlpVideoMeta = serde_json::from_slice(&output.stdout)
        .context("parsing yt-dlp JSON")?;
    Ok(VideoMeta {
        description: meta.description.unwrap_or_default(),
        published_at: meta.upload_date.as_deref().and_then(parse_upload_date),
    })
}

/// Kept for backwards-compat call sites that only need the description.
pub async fn fetch_video_description(youtube_id: &str) -> Result<String> {
    Ok(fetch_video_meta(youtube_id).await?.description)
}

/// Sync one channel: run yt-dlp flat-playlist and upsert videos into DB.
pub async fn sync_channel(
    channel_id: &str,
    channel_url: &str,
    db: &Db,
    config: &Config,
    is_first_sync: bool,
) -> Result<()> {
    info!("syncing channel {channel_url} (first={is_first_sync})");

    let mut args = vec![
        "--flat-playlist".to_string(),
        "--print".to_string(),
        "%(id)s\t%(title)s\t%(upload_date)s\t%(channel_id)s".to_string(),
        "--no-warnings".to_string(),
    ];
    if !is_first_sync && config.sync.playlist_items > 0 {
        args.push("--playlist-items".to_string());
        args.push(format!("1:{}", config.sync.playlist_items));
    }
    args.push(channel_url.to_string());

    let mut child = Command::new("yt-dlp")
        .args(&args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .context("spawning yt-dlp for channel sync")?;

    let stdout = child.stdout.take().expect("stdout piped");
    let mut lines = tokio::io::BufReader::new(stdout).lines();
    let now = Utc::now().to_rfc3339();
    let mut count = 0usize;
    let mut new_ids: Vec<String> = Vec::new();
    let mut youtube_channel_id: Option<String> = None;

    while let Ok(Some(line)) = lines.next_line().await {
        if let Some(entry) = parse_flat_playlist_line(&line) {
            // Capture the YouTube channel ID from the first entry that has it
            if youtube_channel_id.is_none() {
                youtube_channel_id = entry.channel_id.clone();
            }
            let is_new = db.upsert_video(
                &entry.youtube_id,
                channel_id,
                &entry.title,
                entry.published_at.as_deref(),
                &now,
            )
            .context("upserting video")?;
            if is_new {
                new_ids.push(entry.youtube_id.clone());
            }
            count += 1;
        }
    }

    child.wait().await.context("waiting for yt-dlp")?;
    db.set_channel_synced(channel_id, &now)?;
    if let Some(yt_id) = &youtube_channel_id {
        if let Err(e) = db.set_channel_youtube_id(channel_id, yt_id) {
            warn!("set_channel_youtube_id for {channel_id}: {e:#}");
        }
    }
    info!("synced {count} videos for {channel_url} ({} new)", new_ids.len());

    // Fetch full metadata for newly discovered videos.
    // yt-dlp -j gives the real upload_date (flat-playlist is unreliable for dates).
    for youtube_id in &new_ids {
        match fetch_video_meta(youtube_id).await {
            Ok(meta) => {
                if let Err(e) = db.set_video_description(youtube_id, &meta.description) {
                    warn!("set_video_description for {youtube_id}: {e:#}");
                }
                if let Some(date) = &meta.published_at {
                    if let Err(e) = db.set_video_published_at(youtube_id, date) {
                        warn!("set_video_published_at for {youtube_id}: {e:#}");
                    }
                }
            }
            Err(e) => warn!("meta fetch for {youtube_id}: {e:#}"),
        }
    }
    if !new_ids.is_empty() {
        info!("fetched descriptions for {} new videos in {channel_url}", new_ids.len());
    }

    Ok(())
}

/// Walk base_path, mark present videos as downloaded, and clear the downloaded
/// status of any video whose file no longer exists on disk.
pub fn scan_filesystem(base_path: &str, db: &Db) -> Result<()> {
    use std::collections::HashSet;

    let now = Utc::now().to_rfc3339();
    let mut found_ids: HashSet<String> = HashSet::new();
    let mut marked = 0usize;

    for entry in walkdir::WalkDir::new(base_path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        if let Some(youtube_id) = extract_youtube_id_from_path(entry.path()) {
            if db.video_exists(&youtube_id)? {
                let path_str = entry.path().to_string_lossy();
                db.set_video_downloaded(&youtube_id, &now, &path_str)?;
                found_ids.insert(youtube_id);
                marked += 1;
            }
        }
    }

    // Clear downloaded status for any tracked video no longer present on disk
    let previously_downloaded = db.list_downloaded_youtube_ids()?;
    let mut cleared = 0usize;
    for youtube_id in previously_downloaded {
        if !found_ids.contains(&youtube_id) {
            db.clear_video_downloaded(&youtube_id)?;
            cleared += 1;
        }
    }

    info!("filesystem scan: {marked} present, {cleared} stale entries cleared");
    Ok(())
}

/// Background sync loop — runs forever, cycling through all channels every interval_hours.
pub async fn run_sync_loop(db: std::sync::Arc<Db>, config: std::sync::Arc<std::sync::RwLock<Config>>) {
    loop {
        let (interval_hours, base_path) = {
            let cfg = config.read().unwrap();
            (cfg.sync.interval_hours, cfg.output.base_path.clone())
        };

        let channels = db.list_channels().unwrap_or_default();
        for channel in channels {
            let is_first = channel.last_synced_at.is_none();
            let cfg = config.read().unwrap().clone();
            if let Err(e) = sync_channel(&channel.id, &channel.youtube_channel_url, &db, &cfg, is_first).await {
                error!("sync failed for {}: {e:#}", channel.youtube_channel_url);
            }
        }

        if let Err(e) = scan_filesystem(&base_path, &db) {
            warn!("filesystem scan failed: {e:#}");
        }

        tokio::time::sleep(Duration::from_secs(interval_hours * 3600)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn parse_flat_playlist_line_happy_path() {
        let entry = parse_flat_playlist_line("dQw4w9WgXcQ\tNever Gonna Give You Up\t19870727\tUCxxxxxx");
        assert_eq!(
            entry,
            Some(FlatPlaylistEntry {
                youtube_id: "dQw4w9WgXcQ".into(),
                title: "Never Gonna Give You Up".into(),
                published_at: Some("1987-07-27".into()),
                channel_id: Some("UCxxxxxx".into()),
            })
        );
    }

    #[test]
    fn parse_flat_playlist_line_missing_date() {
        let entry = parse_flat_playlist_line("abc123\tSome Video\tNA\tNA");
        assert_eq!(
            entry,
            Some(FlatPlaylistEntry {
                youtube_id: "abc123".into(),
                title: "Some Video".into(),
                published_at: None,
                channel_id: None,
            })
        );
    }

    #[test]
    fn parse_flat_playlist_line_with_channel_id() {
        let entry = parse_flat_playlist_line(
            "lW4FetrdEK4\tSome Title\t20260328\tUCHnyfMqiRRz1Pbc3OkCkEug",
        );
        assert_eq!(
            entry,
            Some(FlatPlaylistEntry {
                youtube_id: "lW4FetrdEK4".into(),
                title: "Some Title".into(),
                published_at: Some("2026-03-28".into()),
                channel_id: Some("UCHnyfMqiRRz1Pbc3OkCkEug".into()),
            })
        );
    }

    #[test]
    fn parse_flat_playlist_line_blank() {
        assert_eq!(parse_flat_playlist_line(""), None);
        assert_eq!(parse_flat_playlist_line("   "), None);
    }

    #[test]
    fn extract_youtube_id_finds_bracket_suffix() {
        let path = Path::new("/plex/Chan/2026-04-05 - Some Video [dQw4w9WgXcQ].mp4");
        assert_eq!(
            extract_youtube_id_from_path(path),
            Some("dQw4w9WgXcQ".into())
        );
    }

    #[test]
    fn extract_youtube_id_returns_none_without_bracket() {
        let path = Path::new("/plex/Chan/2026-04-05 - Some Video.mp4");
        assert_eq!(extract_youtube_id_from_path(path), None);
    }

    #[test]
    fn extract_youtube_id_handles_multiple_brackets() {
        // Should use the last [...]
        let path = Path::new("/plex/Chan/Video [playlist] [abc123def45].mp4");
        assert_eq!(
            extract_youtube_id_from_path(path),
            Some("abc123def45".into())
        );
    }

    #[test]
    fn parse_description_from_json() {
        let json = r#"{"id":"abc","title":"Test","description":"Hello world"}"#;
        let meta: YtDlpVideoMeta = serde_json::from_str(json).unwrap();
        assert_eq!(meta.description.as_deref(), Some("Hello world"));
    }

    #[test]
    fn parse_description_missing_returns_none() {
        let json = r#"{"id":"abc","title":"Test"}"#;
        let meta: YtDlpVideoMeta = serde_json::from_str(json).unwrap();
        assert!(meta.description.is_none());
    }
}
