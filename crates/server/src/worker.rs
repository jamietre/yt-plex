use anyhow::{Context, Result};
use chrono::Utc;
use serde::Deserialize;
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio::io::{AsyncBufReadExt, AsyncReadExt};
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

/// Extract download percentage from a yt-dlp stderr progress line.
/// Matches lines like: `[download]  23.5% of 45.23MiB at 2.34MiB/s ETA 00:16`
pub fn parse_progress(line: &str) -> Option<f32> {
    let line = line.trim();
    let rest = line.strip_prefix("[download]")?.trim();
    let pct_str = rest.split('%').next()?.trim();
    pct_str.parse::<f32>().ok()
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

    // Run yt-dlp, streaming stderr for progress updates
    let mut child = Command::new("yt-dlp")
        .args(["--newline", "--print-json", "-o", &out_template, &job.url])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("spawning yt-dlp (is it installed?)")?;

    // Drain stdout in background to prevent pipe-buffer deadlock with stderr
    let stdout_pipe = child.stdout.take().expect("stdout piped");
    let stdout_task = tokio::spawn(async move {
        let mut buf = Vec::new();
        tokio::io::BufReader::new(stdout_pipe)
            .read_to_end(&mut buf)
            .await
            .ok();
        buf
    });

    let stderr_pipe = child.stderr.take().expect("stderr piped");
    let mut stderr_lines = tokio::io::BufReader::new(stderr_pipe).lines();
    let mut stderr_buf = String::new();

    // Stream stderr: parse progress lines and broadcast, accumulate the rest
    while let Ok(Some(line)) = stderr_lines.next_line().await {
        if let Some(pct) = parse_progress(&line) {
            hub.broadcast(&yt_plex_common::models::WsMessage {
                job_id: job.id.clone(),
                status: JobStatus::Downloading,
                channel_name: None,
                title: None,
                error: None,
                progress: Some(pct),
            });
        }
        stderr_buf.push_str(&line);
        stderr_buf.push('\n');
    }

    let stdout_bytes = stdout_task.await.context("reading yt-dlp stdout")?;
    let exit_status = child.wait().await.context("waiting for yt-dlp")?;

    if !exit_status.success() {
        db.update_job(&job.id, JobStatus::Failed, None, None, Some(&stderr_buf))?;
        let updated = db.get_job(&job.id)?.unwrap();
        hub.broadcast(&yt_plex_common::models::WsMessage::from_job(&updated));
        warn!("yt-dlp failed for {}: {stderr_buf}", job.id);
        return Ok(());
    }

    let stdout = String::from_utf8_lossy(&stdout_bytes).into_owned();
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

    #[test]
    fn parse_progress_extracts_percentage() {
        assert_eq!(
            parse_progress("[download]  23.5% of 45.23MiB at 2.34MiB/s ETA 00:16"),
            Some(23.5)
        );
        assert_eq!(
            parse_progress("[download] 100% of 10.00MiB at 5.00MiB/s ETA 00:00"),
            Some(100.0)
        );
        assert_eq!(parse_progress("[download] Destination: video.mp4"), None);
        assert_eq!(parse_progress("[info] some other line"), None);
        assert_eq!(parse_progress("  0.0% something"), None); // no [download] prefix
    }
}
