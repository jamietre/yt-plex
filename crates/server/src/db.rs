use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use yt_plex_common::models::{Channel, Job, JobStatus, Video, VideoFilter, VideoStatus};

#[derive(Clone)]
pub struct Db {
    pub conn: Arc<Mutex<Connection>>,
}

impl Db {
    pub fn open(path: &str) -> Result<Self> {
        let conn =
            Connection::open(path).with_context(|| format!("opening database: {path}"))?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn insert_job(&self, url: &str) -> Result<Job> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let now_str = now.to_rfc3339();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO jobs (id, url, status, created_at, updated_at) VALUES (?1, ?2, 'queued', ?3, ?3)",
            rusqlite::params![id, url, now_str],
        )?;
        Ok(Job {
            id,
            url: url.to_owned(),
            status: JobStatus::Queued,
            channel_name: None,
            title: None,
            error: None,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn get_job(&self, id: &str) -> Result<Option<Job>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, url, status, channel_name, title, error, created_at, updated_at FROM jobs WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(rusqlite::params![id], row_to_job)?;
        rows.next().transpose().map_err(Into::into)
    }

    pub fn list_jobs(&self) -> Result<Vec<Job>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, url, status, channel_name, title, error, created_at, updated_at FROM jobs ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], row_to_job)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn update_job(
        &self,
        id: &str,
        status: JobStatus,
        channel_name: Option<&str>,
        title: Option<&str>,
        error: Option<&str>,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE jobs SET status=?1, channel_name=COALESCE(?2, channel_name), title=COALESCE(?3, title), error=?4, updated_at=?5 WHERE id=?6",
            rusqlite::params![status.as_str(), channel_name, title, error, now, id],
        )?;
        Ok(())
    }

    pub fn next_queued_job(&self) -> Result<Option<Job>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, url, status, channel_name, title, error, created_at, updated_at FROM jobs WHERE status='queued' ORDER BY created_at ASC LIMIT 1",
        )?;
        let mut rows = stmt.query_map([], row_to_job)?;
        rows.next().transpose().map_err(Into::into)
    }

    /// Reset any jobs stuck in 'downloading' or 'copying' back to 'queued'.
    /// Called on startup to recover from interrupted runs.
    pub fn reset_interrupted_jobs(&self) -> Result<u64> {
        let now = Utc::now().to_rfc3339();
        let conn = self.conn.lock().unwrap();
        let n = conn.execute(
            "UPDATE jobs SET status='queued', updated_at=?1 WHERE status IN ('downloading', 'copying')",
            rusqlite::params![now],
        )?;
        Ok(n as u64)
    }

    pub fn insert_session(&self, token: &str) -> Result<()> {
        let now = Utc::now();
        let expires_at = now + chrono::Duration::days(7);
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO sessions (token, created_at, expires_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![token, now.to_rfc3339(), expires_at.to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn is_valid_session(&self, token: &str) -> Result<bool> {
        let now = Utc::now().to_rfc3339();
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sessions WHERE token=?1 AND expires_at > ?2",
            rusqlite::params![token, now],
            |r| r.get(0),
        )?;
        Ok(count > 0)
    }

    pub fn delete_session(&self, token: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM sessions WHERE token=?1",
            rusqlite::params![token],
        )?;
        Ok(())
    }

    pub fn insert_channel(&self, url: &str, name: &str) -> Result<Channel> {
        let id = Uuid::new_v4().to_string();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO channels (id, youtube_channel_url, name) VALUES (?1, ?2, ?3)",
            rusqlite::params![id, url, name],
        )?;
        Ok(Channel { id, youtube_channel_url: url.to_owned(), name: name.to_owned(), last_synced_at: None })
    }

    pub fn list_channels(&self) -> Result<Vec<Channel>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, youtube_channel_url, name, last_synced_at FROM channels ORDER BY name ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Channel {
                id: row.get(0)?,
                youtube_channel_url: row.get(1)?,
                name: row.get(2)?,
                last_synced_at: row.get(3)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn get_channel(&self, id: &str) -> Result<Option<Channel>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, youtube_channel_url, name, last_synced_at FROM channels WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(rusqlite::params![id], |row| {
            Ok(Channel {
                id: row.get(0)?,
                youtube_channel_url: row.get(1)?,
                name: row.get(2)?,
                last_synced_at: row.get(3)?,
            })
        })?;
        rows.next().transpose().map_err(Into::into)
    }

    pub fn delete_channel(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM channels WHERE id = ?1", rusqlite::params![id])?;
        Ok(())
    }

    pub fn set_channel_synced(&self, id: &str, synced_at: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE channels SET last_synced_at = ?1 WHERE id = ?2",
            rusqlite::params![synced_at, id],
        )?;
        Ok(())
    }

    pub fn upsert_video(
        &self,
        youtube_id: &str,
        channel_id: &str,
        title: &str,
        published_at: Option<&str>,
        last_seen_at: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO videos (youtube_id, channel_id, title, published_at, last_seen_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(youtube_id) DO UPDATE SET
               title = excluded.title,
               last_seen_at = excluded.last_seen_at",
            rusqlite::params![youtube_id, channel_id, title, published_at, last_seen_at],
        )?;
        Ok(())
    }

    pub fn set_video_downloaded(&self, youtube_id: &str, downloaded_at: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE videos SET downloaded_at = ?1 WHERE youtube_id = ?2",
            rusqlite::params![downloaded_at, youtube_id],
        )?;
        Ok(())
    }

    pub fn ignore_video(&self, youtube_id: &str, ignored_at: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE videos SET ignored_at = ?1 WHERE youtube_id = ?2",
            rusqlite::params![ignored_at, youtube_id],
        )?;
        Ok(())
    }

    pub fn unignore_video(&self, youtube_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE videos SET ignored_at = NULL WHERE youtube_id = ?1",
            rusqlite::params![youtube_id],
        )?;
        Ok(())
    }

    pub fn video_exists(&self, youtube_id: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM videos WHERE youtube_id = ?1",
            rusqlite::params![youtube_id],
            |r| r.get(0),
        )?;
        Ok(count > 0)
    }

    pub fn list_videos_for_channel(
        &self,
        channel_id: &str,
        filter: VideoFilter,
        show_ignored: bool,
    ) -> Result<Vec<Video>> {
        let active_job_subq = "EXISTS(SELECT 1 FROM jobs WHERE url = 'https://www.youtube.com/watch?v=' || v.youtube_id AND status IN ('queued','downloading','copying'))";
        let ignore_cond = if show_ignored { "1=1" } else { "v.ignored_at IS NULL" };

        let filter_cond = match filter {
            VideoFilter::New => format!(
                "NOT {active_job_subq} AND v.downloaded_at IS NULL AND ({ignore_cond})"
            ),
            VideoFilter::Downloaded => format!(
                "({active_job_subq} OR v.downloaded_at IS NOT NULL) AND ({ignore_cond})"
            ),
            VideoFilter::All => ignore_cond.to_string(),
        };

        let sql = format!(
            "SELECT v.youtube_id, v.channel_id, v.title, v.published_at,
                    v.downloaded_at, v.last_seen_at, v.ignored_at,
                    CASE
                        WHEN {active_job_subq} THEN 'in_progress'
                        WHEN v.downloaded_at IS NOT NULL THEN 'downloaded'
                        WHEN v.ignored_at IS NOT NULL THEN 'ignored'
                        ELSE 'new'
                    END as derived_status
             FROM videos v
             WHERE v.channel_id = ?1
               AND ({filter_cond})
             ORDER BY v.published_at DESC NULLS LAST, v.last_seen_at DESC"
        );

        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params![channel_id], |row| {
            let status_str: String = row.get(7)?;
            let status = match status_str.as_str() {
                "in_progress" => VideoStatus::InProgress,
                "downloaded" => VideoStatus::Downloaded,
                "ignored" => VideoStatus::Ignored,
                _ => VideoStatus::New,
            };
            Ok(Video {
                youtube_id: row.get(0)?,
                channel_id: row.get(1)?,
                title: row.get(2)?,
                published_at: row.get(3)?,
                downloaded_at: row.get(4)?,
                last_seen_at: row.get(5)?,
                ignored_at: row.get(6)?,
                status,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }
}

fn row_to_job(row: &rusqlite::Row) -> rusqlite::Result<Job> {
    let status_str: String = row.get(2)?;
    let status = status_str.parse::<JobStatus>().map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(
            2,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())),
        )
    })?;
    let created_at_str: String = row.get(6)?;
    let updated_at_str: String = row.get(7)?;
    Ok(Job {
        id: row.get(0)?,
        url: row.get(1)?,
        status,
        channel_name: row.get(3)?,
        title: row.get(4)?,
        error: row.get(5)?,
        created_at: created_at_str.parse().unwrap_or_else(|_| Utc::now()),
        updated_at: updated_at_str.parse().unwrap_or_else(|_| Utc::now()),
    })
}

const SCHEMA: &str = "
PRAGMA journal_mode=WAL;
PRAGMA foreign_keys=ON;

CREATE TABLE IF NOT EXISTS jobs (
    id           TEXT PRIMARY KEY,
    url          TEXT NOT NULL,
    status       TEXT NOT NULL DEFAULT 'queued',
    channel_name TEXT,
    title        TEXT,
    error        TEXT,
    created_at   TEXT NOT NULL,
    updated_at   TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS sessions (
    token      TEXT PRIMARY KEY,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS channels (
    id                   TEXT PRIMARY KEY,
    youtube_channel_url  TEXT NOT NULL UNIQUE,
    name                 TEXT NOT NULL,
    last_synced_at       TEXT
);

CREATE TABLE IF NOT EXISTS videos (
    youtube_id    TEXT PRIMARY KEY,
    channel_id    TEXT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    title         TEXT NOT NULL,
    published_at  TEXT,
    downloaded_at TEXT,
    last_seen_at  TEXT NOT NULL,
    ignored_at    TEXT
);

CREATE INDEX IF NOT EXISTS idx_videos_channel_id ON videos(channel_id);
CREATE INDEX IF NOT EXISTS idx_videos_published_at ON videos(published_at DESC);
";

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Db {
        Db::open(":memory:").unwrap()
    }

    #[test]
    fn schema_initialises() {
        let db = test_db();
        let conn = db.conn.lock().unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM jobs", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn insert_and_get_job() {
        let db = test_db();
        let job = db.insert_job("https://youtube.com/watch?v=abc").unwrap();
        assert_eq!(job.status, JobStatus::Queued);
        assert_eq!(job.url, "https://youtube.com/watch?v=abc");

        let fetched = db.get_job(&job.id).unwrap().unwrap();
        assert_eq!(fetched.id, job.id);
    }

    #[test]
    fn list_jobs_returns_newest_first() {
        let db = test_db();
        db.insert_job("https://youtube.com/watch?v=1").unwrap();
        // Small sleep to ensure different timestamps
        std::thread::sleep(std::time::Duration::from_millis(10));
        db.insert_job("https://youtube.com/watch?v=2").unwrap();
        let jobs = db.list_jobs().unwrap();
        assert_eq!(jobs.len(), 2);
        assert!(jobs[0].created_at >= jobs[1].created_at);
    }

    #[test]
    fn update_job_status_sets_fields() {
        let db = test_db();
        let job = db.insert_job("https://youtube.com/watch?v=x").unwrap();
        db.update_job(
            &job.id,
            JobStatus::Downloading,
            Some("MyChan"),
            Some("My Title"),
            None,
        )
        .unwrap();
        let updated = db.get_job(&job.id).unwrap().unwrap();
        assert_eq!(updated.status, JobStatus::Downloading);
        assert_eq!(updated.channel_name.as_deref(), Some("MyChan"));
    }

    #[test]
    fn next_queued_job_returns_oldest() {
        let db = test_db();
        let j1 = db.insert_job("https://youtube.com/watch?v=1").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        db.insert_job("https://youtube.com/watch?v=2").unwrap();
        let next = db.next_queued_job().unwrap().unwrap();
        assert_eq!(next.id, j1.id);
    }

    #[test]
    fn insert_and_validate_session() {
        let db = test_db();
        db.insert_session("tok123").unwrap();
        assert!(db.is_valid_session("tok123").unwrap());
        assert!(!db.is_valid_session("bogus").unwrap());
    }

    #[test]
    fn delete_session_invalidates_it() {
        let db = test_db();
        db.insert_session("tok456").unwrap();
        db.delete_session("tok456").unwrap();
        assert!(!db.is_valid_session("tok456").unwrap());
    }

    #[test]
    fn channels_table_exists() {
        let db = test_db();
        let conn = db.conn.lock().unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM channels", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn videos_table_exists() {
        let db = test_db();
        let conn = db.conn.lock().unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM videos", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn insert_and_list_channels() {
        let db = test_db();
        let ch = db.insert_channel("https://youtube.com/@Veritasium", "Veritasium").unwrap();
        assert_eq!(ch.name, "Veritasium");
        assert_eq!(ch.youtube_channel_url, "https://youtube.com/@Veritasium");
        assert!(ch.last_synced_at.is_none());
        let list = db.list_channels().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, ch.id);
    }

    #[test]
    fn get_channel_returns_none_for_missing() {
        let db = test_db();
        assert!(db.get_channel("nonexistent").unwrap().is_none());
    }

    #[test]
    fn delete_channel_removes_it() {
        let db = test_db();
        let ch = db.insert_channel("https://youtube.com/@LTT", "LTT").unwrap();
        db.delete_channel(&ch.id).unwrap();
        assert_eq!(db.list_channels().unwrap().len(), 0);
    }

    #[test]
    fn set_channel_synced_updates_timestamp() {
        let db = test_db();
        let ch = db.insert_channel("https://youtube.com/@test", "Test").unwrap();
        db.set_channel_synced(&ch.id, "2026-04-05T12:00:00Z").unwrap();
        let updated = db.get_channel(&ch.id).unwrap().unwrap();
        assert_eq!(updated.last_synced_at.as_deref(), Some("2026-04-05T12:00:00Z"));
    }

    fn insert_test_channel(db: &Db) -> Channel {
        db.insert_channel("https://youtube.com/@test", "Test").unwrap()
    }

    #[test]
    fn upsert_and_list_videos_new() {
        let db = test_db();
        let ch = insert_test_channel(&db);
        db.upsert_video("abc123", &ch.id, "Test Video", Some("2026-01-01"), "2026-04-05T00:00:00Z").unwrap();
        let videos = db.list_videos_for_channel(&ch.id, VideoFilter::All, false).unwrap();
        assert_eq!(videos.len(), 1);
        assert_eq!(videos[0].youtube_id, "abc123");
        assert_eq!(videos[0].status, VideoStatus::New);
    }

    #[test]
    fn set_video_downloaded_changes_status() {
        let db = test_db();
        let ch = insert_test_channel(&db);
        db.upsert_video("abc123", &ch.id, "Test Video", None, "2026-04-05T00:00:00Z").unwrap();
        db.set_video_downloaded("abc123", "2026-04-05T12:00:00Z").unwrap();
        let new_videos = db.list_videos_for_channel(&ch.id, VideoFilter::New, false).unwrap();
        assert_eq!(new_videos.len(), 0);
        let downloaded = db.list_videos_for_channel(&ch.id, VideoFilter::Downloaded, false).unwrap();
        assert_eq!(downloaded.len(), 1);
        assert_eq!(downloaded[0].status, VideoStatus::Downloaded);
    }

    #[test]
    fn ignore_hides_from_new_filter() {
        let db = test_db();
        let ch = insert_test_channel(&db);
        db.upsert_video("xyz789", &ch.id, "Another Video", None, "2026-04-05T00:00:00Z").unwrap();
        db.ignore_video("xyz789", "2026-04-05T12:00:00Z").unwrap();
        let new_videos = db.list_videos_for_channel(&ch.id, VideoFilter::New, false).unwrap();
        assert_eq!(new_videos.len(), 0);
        let with_ignored = db.list_videos_for_channel(&ch.id, VideoFilter::New, true).unwrap();
        assert_eq!(with_ignored.len(), 1);
        assert_eq!(with_ignored[0].status, VideoStatus::Ignored);
    }

    #[test]
    fn unignore_makes_video_new_again() {
        let db = test_db();
        let ch = insert_test_channel(&db);
        db.upsert_video("vid1", &ch.id, "Video", None, "2026-04-05T00:00:00Z").unwrap();
        db.ignore_video("vid1", "2026-04-05T12:00:00Z").unwrap();
        db.unignore_video("vid1").unwrap();
        let new_videos = db.list_videos_for_channel(&ch.id, VideoFilter::New, false).unwrap();
        assert_eq!(new_videos.len(), 1);
        assert_eq!(new_videos[0].status, VideoStatus::New);
    }

    #[test]
    fn videos_ordered_by_published_at_desc() {
        let db = test_db();
        let ch = insert_test_channel(&db);
        db.upsert_video("old", &ch.id, "Old Video", Some("2025-01-01"), "2026-04-05T00:00:00Z").unwrap();
        db.upsert_video("new", &ch.id, "New Video", Some("2026-01-01"), "2026-04-05T00:00:00Z").unwrap();
        let videos = db.list_videos_for_channel(&ch.id, VideoFilter::All, false).unwrap();
        assert_eq!(videos[0].youtube_id, "new");
        assert_eq!(videos[1].youtube_id, "old");
    }

    #[test]
    fn all_filter_excludes_ignored_by_default() {
        let db = test_db();
        let ch = insert_test_channel(&db);
        db.upsert_video("v1", &ch.id, "V1", None, "2026-04-05T00:00:00Z").unwrap();
        db.upsert_video("v2", &ch.id, "V2", None, "2026-04-05T00:00:00Z").unwrap();
        db.ignore_video("v2", "2026-04-05T12:00:00Z").unwrap();
        let all = db.list_videos_for_channel(&ch.id, VideoFilter::All, false).unwrap();
        assert_eq!(all.len(), 1);
        let all_with_ignored = db.list_videos_for_channel(&ch.id, VideoFilter::All, true).unwrap();
        assert_eq!(all_with_ignored.len(), 2);
    }
}
