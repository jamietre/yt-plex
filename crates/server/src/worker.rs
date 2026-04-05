use anyhow::{Context, Result};
use chrono::Utc;
use serde::Deserialize;
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio::process::Command;
use tracing::{error, info, warn};
use yt_plex_common::{config::Config, models::JobStatus};

use crate::{db::Db, plex, template, ws::WsHub};

#[derive(Debug, Deserialize)]
pub struct YtDlpMeta {
    pub channel: String,
    pub title: String,
    pub ext: String,
    pub id: String,
}

pub fn parse_ytdlp_json(json: &str) -> Result<YtDlpMeta> {
    serde_json::from_str(json).context("parsing yt-dlp JSON output")
}

/// Spawns the background download loop. Runs forever until the process exits.
pub async fn run(db: Arc<Db>, config: Arc<std::sync::RwLock<Config>>, hub: WsHub) {
    loop {
        if let Err(e) = tick(&db, &config, &hub).await {
            error!("worker error: {e:#}");
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

async fn tick(
    db: &Db,
    config: &std::sync::RwLock<Config>,
    hub: &WsHub,
) -> Result<()> {
    let Some(job) = db.next_queued_job()? else {
        return Ok(());
    };

    info!("starting download: {} ({})", job.id, job.url);

    // Mark as downloading
    db.update_job(&job.id, JobStatus::Downloading, None, None, None)?;
    let updated = db.get_job(&job.id)?.unwrap();
    hub.broadcast(&yt_plex_common::models::WsMessage::from_job(&updated));

    let tmp = tempfile::tempdir()?;
    let out_template = tmp
        .path()
        .join("%(id)s.%(ext)s")
        .to_string_lossy()
        .into_owned();

    // Run yt-dlp
    let output = Command::new("yt-dlp")
        .args(["--print-json", "-o", &out_template, &job.url])
        .output()
        .await
        .context("spawning yt-dlp (is it installed?)")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        db.update_job(&job.id, JobStatus::Failed, None, None, Some(&stderr))?;
        let updated = db.get_job(&job.id)?.unwrap();
        hub.broadcast(&yt_plex_common::models::WsMessage::from_job(&updated));
        warn!("yt-dlp failed for {}: {stderr}", job.id);
        return Ok(());
    }

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    // yt-dlp may print multiple JSON lines (playlist); take the last non-empty one
    let last_line = stdout
        .lines()
        .filter(|l| !l.trim().is_empty())
        .last()
        .unwrap_or("");

    let meta = match parse_ytdlp_json(last_line) {
        Ok(m) => m,
        Err(e) => {
            let msg = format!("parsing yt-dlp output: {e}");
            db.update_job(&job.id, JobStatus::Failed, None, None, Some(&msg))?;
            let updated = db.get_job(&job.id)?.unwrap();
            hub.broadcast(&yt_plex_common::models::WsMessage::from_job(&updated));
            return Ok(());
        }
    };

    let date = Utc::now().format("%Y-%m-%d").to_string();
    let (base_path, path_template) = {
        let cfg = config.read().unwrap();
        (cfg.output.base_path.clone(), cfg.output.path_template.clone())
    };

    let rel_path = template::render(&path_template, &meta.channel, &date, &meta.title, &meta.ext);
    let dest: PathBuf = PathBuf::from(&base_path).join(&rel_path);
    let src: PathBuf = tmp.path().join(format!("{}.{}", meta.id, meta.ext));

    // Mark as copying
    db.update_job(
        &job.id,
        JobStatus::Copying,
        Some(&meta.channel),
        Some(&meta.title),
        None,
    )?;
    let updated = db.get_job(&job.id)?.unwrap();
    hub.broadcast(&yt_plex_common::models::WsMessage::from_job(&updated));

    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("creating dir: {}", parent.display()))?;
    }
    // rename(2) fails across devices; fall back to copy-then-delete
    if let Err(e) = tokio::fs::rename(&src, &dest).await {
        if e.raw_os_error() == Some(18) {
            // EXDEV: cross-device link
            tokio::fs::copy(&src, &dest)
                .await
                .with_context(|| format!("copying {} → {}", src.display(), dest.display()))?;
            tokio::fs::remove_file(&src).await.ok();
        } else {
            return Err(e)
                .with_context(|| format!("moving {} → {}", src.display(), dest.display()));
        }
    }

    db.update_job(&job.id, JobStatus::Done, None, None, None)?;
    let updated = db.get_job(&job.id)?.unwrap();
    hub.broadcast(&yt_plex_common::models::WsMessage::from_job(&updated));
    info!("done: {} → {}", job.url, dest.display());

    // Trigger Plex refresh
    let plex_cfg = {
        config.read().unwrap().plex.clone()
    };
    if let Err(e) = plex::trigger_library_refresh(&plex_cfg).await {
        warn!("Plex refresh failed: {e:#}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ytdlp_output_extracts_fields() {
        let json =
            r#"{"channel": "MyChan", "title": "My Video", "ext": "mp4", "id": "abc123"}"#;
        let meta = parse_ytdlp_json(json).unwrap();
        assert_eq!(meta.channel, "MyChan");
        assert_eq!(meta.title, "My Video");
        assert_eq!(meta.ext, "mp4");
        assert_eq!(meta.id, "abc123");
    }
}
