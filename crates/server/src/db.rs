use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use yt_plex_common::models::{Channel, Job, JobStatus, Profile, Video, VideoFilter, VideoPage, VideoStatus};

#[derive(Clone)]
pub struct Db {
    pub conn: Arc<Mutex<Connection>>,
}

impl Db {
    pub fn open(path: &str) -> Result<Self> {
        let conn =
            Connection::open(path).with_context(|| format!("opening database: {path}"))?;
        run_migrations(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn insert_job(&self, url: &str, channel_name: Option<&str>, title: Option<&str>) -> Result<Job> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let now_str = now.to_rfc3339();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO jobs (id, url, status, channel_name, title, created_at, updated_at)
             VALUES (?1, ?2, 'queued', ?3, ?4, ?5, ?5)",
            rusqlite::params![id, url, channel_name, title, now_str],
        )?;
        Ok(Job {
            id,
            url: url.to_owned(),
            status: JobStatus::Queued,
            channel_name: channel_name.map(str::to_owned),
            title: title.map(str::to_owned),
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

    /// Upsert a video. Returns `true` if the row was newly inserted, `false` if it already existed.
    pub fn upsert_video(
        &self,
        youtube_id: &str,
        channel_id: &str,
        title: &str,
        published_at: Option<&str>,
        last_seen_at: &str,
    ) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let already_exists: bool = conn
            .query_row(
                "SELECT 1 FROM videos WHERE youtube_id = ?1",
                rusqlite::params![youtube_id],
                |_| Ok(true),
            )
            .unwrap_or(false);
        let is_new = !already_exists;
        conn.execute(
            "INSERT INTO videos (youtube_id, channel_id, title, published_at, last_seen_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(youtube_id) DO UPDATE SET
               title = excluded.title,
               published_at = COALESCE(excluded.published_at, videos.published_at),
               last_seen_at = excluded.last_seen_at",
            rusqlite::params![youtube_id, channel_id, title, published_at, last_seen_at],
        )?;
        Ok(is_new)
    }

    pub fn set_video_downloaded(&self, youtube_id: &str, downloaded_at: &str, file_path: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE videos SET downloaded_at = ?1, file_path = ?2 WHERE youtube_id = ?3",
            rusqlite::params![downloaded_at, file_path, youtube_id],
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

    pub fn get_video(&self, youtube_id: &str) -> Result<Option<Video>> {
        self.get_video_for_profile(youtube_id, None)
    }

    pub fn get_video_for_profile(&self, youtube_id: &str, profile_id: Option<i64>) -> Result<Option<Video>> {
        let active_job_subq = "EXISTS(SELECT 1 FROM jobs WHERE url = 'https://www.youtube.com/watch?v=' || v.youtube_id AND status IN ('queued','downloading','copying'))";
        let profile_ignored_subq = "EXISTS(SELECT 1 FROM profile_video_ignores WHERE profile_id=?2 AND youtube_id=v.youtube_id)";
        let sql = format!(
            "SELECT v.youtube_id, v.channel_id, v.title, v.published_at,
                    v.downloaded_at, v.last_seen_at, v.ignored_at,
                    CASE
                        WHEN {active_job_subq} THEN 'in_progress'
                        WHEN v.downloaded_at IS NOT NULL THEN 'downloaded'
                        WHEN v.ignored_at IS NOT NULL THEN 'ignored'
                        WHEN ?2 IS NOT NULL AND {profile_ignored_subq} THEN 'ignored'
                        ELSE 'new'
                    END as derived_status,
                    v.description,
                    v.file_path
             FROM videos v
             WHERE v.youtube_id = ?1"
        );
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query_map(rusqlite::params![youtube_id, profile_id], |row| {
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
                description: row.get(8)?,
                file_path: row.get(9)?,
            })
        })?;
        rows.next().transpose().map_err(Into::into)
    }

    pub fn set_video_description(&self, youtube_id: &str, description: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE videos SET description = ?1 WHERE youtube_id = ?2",
            rusqlite::params![description, youtube_id],
        )?;
        Ok(())
    }

    pub fn list_videos_for_channel(
        &self,
        channel_id: &str,
        filter: VideoFilter,
        show_ignored: bool,
        search: Option<&str>,
        limit: usize,
        offset: usize,
        profile_id: Option<i64>,
    ) -> Result<VideoPage> {
        let active_job_subq = "EXISTS(SELECT 1 FROM jobs WHERE url = 'https://www.youtube.com/watch?v=' || v.youtube_id AND status IN ('queued','downloading','copying'))";
        let profile_ignored_subq = "EXISTS(SELECT 1 FROM profile_video_ignores WHERE profile_id=?5 AND youtube_id=v.youtube_id)";
        let ignore_cond = if show_ignored {
            "1=1".to_string()
        } else if profile_id.is_some() {
            format!("v.ignored_at IS NULL AND NOT ({profile_ignored_subq})")
        } else {
            "v.ignored_at IS NULL".to_string()
        };

        let filter_cond = match filter {
            VideoFilter::New => format!(
                "NOT {active_job_subq} AND v.downloaded_at IS NULL AND ({ignore_cond})"
            ),
            VideoFilter::Downloaded => format!(
                "({active_job_subq} OR v.downloaded_at IS NOT NULL) AND ({ignore_cond})"
            ),
            VideoFilter::All => ignore_cond.to_string(),
        };

        // ?2 IS NULL short-circuits before MATCH is evaluated, so NULL search is safe.
        // ?5 is profile_id (may be NULL for admin).
        let sql = format!(
            "SELECT v.youtube_id, v.channel_id, v.title, v.published_at,
                    v.downloaded_at, v.last_seen_at, v.ignored_at,
                    CASE
                        WHEN {active_job_subq} THEN 'in_progress'
                        WHEN v.downloaded_at IS NOT NULL THEN 'downloaded'
                        WHEN v.ignored_at IS NOT NULL THEN 'ignored'
                        WHEN ?5 IS NOT NULL AND {profile_ignored_subq} THEN 'ignored'
                        ELSE 'new'
                    END as derived_status,
                    v.description,
                    v.file_path
             FROM videos v
             WHERE v.channel_id = ?1
               AND (?2 IS NULL OR v.rowid IN (SELECT rowid FROM videos_fts WHERE videos_fts MATCH ('title:' || ?2)))
               AND ({filter_cond})
             ORDER BY v.published_at DESC NULLS LAST, v.last_seen_at DESC
             LIMIT ?3 OFFSET ?4"
        );

        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(&sql)?;
        // Fetch limit+1 to detect whether there are more pages.
        let fetch_limit = (limit + 1) as i64;
        let rows = stmt.query_map(
            rusqlite::params![channel_id, search, fetch_limit, offset as i64, profile_id],
            |row| {
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
                    description: row.get(8)?,
                    file_path: row.get(9)?,
                })
            },
        )?;
        let mut videos: Vec<Video> = rows.collect::<Result<_, _>>()?;
        let has_more = videos.len() > limit;
        if has_more {
            videos.pop();
        }
        Ok(VideoPage { videos, has_more })
    }

    // ── Profile methods ────────────────────────────────────────────────────────

    fn row_to_profile(row: &rusqlite::Row) -> rusqlite::Result<Profile> {
        Ok(Profile {
            id: row.get(0)?,
            name: row.get(1)?,
            linked_email: row.get(2)?,
            is_admin_profile: row.get::<_, i64>(3)? != 0,
            created_at: row.get(4)?,
        })
    }

    pub fn list_profiles(&self, include_admin: bool) -> Result<Vec<Profile>> {
        let conn = self.conn.lock().unwrap();
        let sql = if include_admin {
            "SELECT id, name, linked_email, is_admin_profile, created_at FROM profiles ORDER BY name ASC"
        } else {
            "SELECT id, name, linked_email, is_admin_profile, created_at FROM profiles WHERE is_admin_profile=0 ORDER BY name ASC"
        };
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map([], Self::row_to_profile)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn create_profile(&self, name: &str, linked_email: Option<&str>, is_admin: bool) -> Result<Profile> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO profiles (name, linked_email, is_admin_profile) VALUES (?1, ?2, ?3)",
            rusqlite::params![name, linked_email, is_admin as i64],
        )?;
        let id = conn.last_insert_rowid();
        let created_at: String = conn.query_row(
            "SELECT created_at FROM profiles WHERE id=?1",
            rusqlite::params![id],
            |r| r.get(0),
        )?;
        Ok(Profile {
            id,
            name: name.to_owned(),
            linked_email: linked_email.map(str::to_owned),
            is_admin_profile: is_admin,
            created_at,
        })
    }

    pub fn delete_profile(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM profiles WHERE id=?1", rusqlite::params![id])?;
        Ok(())
    }

    pub fn get_profile(&self, id: i64) -> Result<Option<Profile>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, linked_email, is_admin_profile, created_at FROM profiles WHERE id=?1",
        )?;
        let mut rows = stmt.query_map(rusqlite::params![id], Self::row_to_profile)?;
        rows.next().transpose().map_err(Into::into)
    }

    pub fn get_profile_by_email(&self, email: &str) -> Result<Option<Profile>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, linked_email, is_admin_profile, created_at FROM profiles WHERE linked_email=?1",
        )?;
        let mut rows = stmt.query_map(rusqlite::params![email], Self::row_to_profile)?;
        rows.next().transpose().map_err(Into::into)
    }

    pub fn subscribe_channel(&self, profile_id: i64, channel_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO profile_channels (profile_id, channel_id) VALUES (?1, ?2)",
            rusqlite::params![profile_id, channel_id],
        )?;
        Ok(())
    }

    pub fn unsubscribe_channel(&self, profile_id: i64, channel_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM profile_channels WHERE profile_id=?1 AND channel_id=?2",
            rusqlite::params![profile_id, channel_id],
        )?;
        Ok(())
    }

    pub fn list_profile_channel_ids(&self, profile_id: i64) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT channel_id FROM profile_channels WHERE profile_id=?1",
        )?;
        let rows = stmt.query_map(rusqlite::params![profile_id], |r| r.get(0))?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn list_channels_for_profile(&self, profile_id: Option<i64>) -> Result<Vec<Channel>> {
        let conn = self.conn.lock().unwrap();
        let sql = match profile_id {
            None => "SELECT id, youtube_channel_url, name, last_synced_at FROM channels ORDER BY name ASC".to_string(),
            Some(pid) => format!(
                "SELECT id, youtube_channel_url, name, last_synced_at FROM channels
                 WHERE id IN (SELECT channel_id FROM profile_channels WHERE profile_id={pid})
                 ORDER BY name ASC"
            ),
        };
        let mut stmt = conn.prepare(&sql)?;
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

    pub fn ignore_video_for_profile(&self, profile_id: i64, youtube_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO profile_video_ignores (profile_id, youtube_id) VALUES (?1, ?2)",
            rusqlite::params![profile_id, youtube_id],
        )?;
        Ok(())
    }

    pub fn unignore_video_for_profile(&self, profile_id: i64, youtube_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM profile_video_ignores WHERE profile_id=?1 AND youtube_id=?2",
            rusqlite::params![profile_id, youtube_id],
        )?;
        Ok(())
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

// ── Database migrations ───────────────────────────────────────────────────────
//
// Rules:
//   1. NEVER edit an existing migration — only append new ones.
//   2. Each migration is applied exactly once; the applied version is stored
//      in SQLite's built-in `PRAGMA user_version`.
//   3. Additive column changes must use `ALTER TABLE … ADD COLUMN`.
//   4. The pragmas block runs on every open (not versioned).

const PRAGMAS: &str = "
PRAGMA journal_mode=WAL;
PRAGMA foreign_keys=ON;
";

/// Ordered list of migrations. Index 0 = migration version 1.
/// Never modify entries already in this list — only append.
const MIGRATIONS: &[&str] = &[
    // ── v1: initial schema ────────────────────────────────────────────────────
    "
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
        ignored_at    TEXT,
        description   TEXT,
        file_path     TEXT
    );

    CREATE INDEX IF NOT EXISTS idx_videos_channel_id ON videos(channel_id);
    CREATE INDEX IF NOT EXISTS idx_videos_published_at ON videos(published_at DESC);

    CREATE VIRTUAL TABLE IF NOT EXISTS videos_fts USING fts5(
        title,
        description,
        content=videos,
        content_rowid=rowid
    );

    CREATE TRIGGER IF NOT EXISTS videos_fts_insert AFTER INSERT ON videos BEGIN
        INSERT INTO videos_fts(rowid, title, description)
        VALUES (new.rowid, new.title, new.description);
    END;

    CREATE TRIGGER IF NOT EXISTS videos_fts_update AFTER UPDATE ON videos BEGIN
        INSERT INTO videos_fts(videos_fts, rowid, title, description)
        VALUES ('delete', old.rowid, old.title, old.description);
        INSERT INTO videos_fts(rowid, title, description)
        VALUES (new.rowid, new.title, new.description);
    END;

    CREATE TRIGGER IF NOT EXISTS videos_fts_delete AFTER DELETE ON videos BEGIN
        INSERT INTO videos_fts(videos_fts, rowid, title, description)
        VALUES ('delete', old.rowid, old.title, old.description);
    END;

    CREATE TABLE IF NOT EXISTS profiles (
        id               INTEGER PRIMARY KEY,
        name             TEXT    NOT NULL UNIQUE,
        linked_email     TEXT,
        is_admin_profile INTEGER NOT NULL DEFAULT 0,
        created_at       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
    );

    CREATE TABLE IF NOT EXISTS profile_channels (
        profile_id  INTEGER NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
        channel_id  TEXT    NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
        PRIMARY KEY (profile_id, channel_id)
    );

    CREATE TABLE IF NOT EXISTS profile_video_ignores (
        profile_id  INTEGER NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
        youtube_id  TEXT    NOT NULL,
        ignored_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
        PRIMARY KEY (profile_id, youtube_id)
    );
    ",
    // ── v2: add your next migration here ─────────────────────────────────────
    // "ALTER TABLE ... ADD COLUMN ...;",
];

fn run_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch(PRAGMAS)?;

    let current: i64 = conn
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .context("reading user_version")?;

    for (i, sql) in MIGRATIONS.iter().enumerate() {
        let version = (i + 1) as i64;
        if current < version {
            conn.execute_batch(sql)
                .with_context(|| format!("applying migration v{version}"))?;
            // PRAGMA user_version cannot use bound parameters
            conn.execute_batch(&format!("PRAGMA user_version = {version};"))?;
        }
    }
    Ok(())
}

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
        let job = db.insert_job("https://youtube.com/watch?v=abc", None, None).unwrap();
        assert_eq!(job.status, JobStatus::Queued);
        assert_eq!(job.url, "https://youtube.com/watch?v=abc");

        let fetched = db.get_job(&job.id).unwrap().unwrap();
        assert_eq!(fetched.id, job.id);
    }

    #[test]
    fn list_jobs_returns_newest_first() {
        let db = test_db();
        db.insert_job("https://youtube.com/watch?v=test", None, None).unwrap();
        // Small sleep to ensure different timestamps
        std::thread::sleep(std::time::Duration::from_millis(10));
        db.insert_job("https://youtube.com/watch?v=test", None, None).unwrap();
        let jobs = db.list_jobs().unwrap();
        assert_eq!(jobs.len(), 2);
        assert!(jobs[0].created_at >= jobs[1].created_at);
    }

    #[test]
    fn update_job_status_sets_fields() {
        let db = test_db();
        let job = db.insert_job("https://youtube.com/watch?v=test", None, None).unwrap();
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
        let j1 = db.insert_job("https://youtube.com/watch?v=test", None, None).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        db.insert_job("https://youtube.com/watch?v=test", None, None).unwrap();
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
        let page = db.list_videos_for_channel(&ch.id, VideoFilter::All, false, None, 50, 0, None).unwrap();
        let videos = page.videos;
        assert_eq!(videos.len(), 1);
        assert_eq!(videos[0].youtube_id, "abc123");
        assert_eq!(videos[0].status, VideoStatus::New);
    }

    #[test]
    fn set_video_downloaded_changes_status() {
        let db = test_db();
        let ch = insert_test_channel(&db);
        db.upsert_video("abc123", &ch.id, "Test Video", None, "2026-04-05T00:00:00Z").unwrap();
        db.set_video_downloaded("abc123", "2026-04-05T12:00:00Z", "/tmp/test.mp4").unwrap();
        let page = db.list_videos_for_channel(&ch.id, VideoFilter::New, false, None, 50, 0, None).unwrap();
        let new_videos = page.videos;
        assert_eq!(new_videos.len(), 0);
        let page = db.list_videos_for_channel(&ch.id, VideoFilter::Downloaded, false, None, 50, 0, None).unwrap();
        let downloaded = page.videos;
        assert_eq!(downloaded.len(), 1);
        assert_eq!(downloaded[0].status, VideoStatus::Downloaded);
    }

    #[test]
    fn ignore_hides_from_new_filter() {
        let db = test_db();
        let ch = insert_test_channel(&db);
        db.upsert_video("xyz789", &ch.id, "Another Video", None, "2026-04-05T00:00:00Z").unwrap();
        db.ignore_video("xyz789", "2026-04-05T12:00:00Z").unwrap();
        let page = db.list_videos_for_channel(&ch.id, VideoFilter::New, false, None, 50, 0, None).unwrap();
        let new_videos = page.videos;
        assert_eq!(new_videos.len(), 0);
        let page = db.list_videos_for_channel(&ch.id, VideoFilter::New, true, None, 50, 0, None).unwrap();
        let with_ignored = page.videos;
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
        let page = db.list_videos_for_channel(&ch.id, VideoFilter::New, false, None, 50, 0, None).unwrap();
        let new_videos = page.videos;
        assert_eq!(new_videos.len(), 1);
        assert_eq!(new_videos[0].status, VideoStatus::New);
    }

    #[test]
    fn videos_ordered_by_published_at_desc() {
        let db = test_db();
        let ch = insert_test_channel(&db);
        db.upsert_video("old", &ch.id, "Old Video", Some("2025-01-01"), "2026-04-05T00:00:00Z").unwrap();
        db.upsert_video("new", &ch.id, "New Video", Some("2026-01-01"), "2026-04-05T00:00:00Z").unwrap();
        let page = db.list_videos_for_channel(&ch.id, VideoFilter::All, false, None, 50, 0, None).unwrap();
        let videos = page.videos;
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
        let page = db.list_videos_for_channel(&ch.id, VideoFilter::All, false, None, 50, 0, None).unwrap();
        let all = page.videos;
        assert_eq!(all.len(), 1);
        let page = db.list_videos_for_channel(&ch.id, VideoFilter::All, true, None, 50, 0, None).unwrap();
        let all_with_ignored = page.videos;
        assert_eq!(all_with_ignored.len(), 2);
    }

    #[test]
    fn video_exists_returns_correct_bool() {
        let db = test_db();
        let ch = insert_test_channel(&db);
        assert!(!db.video_exists("abc123").unwrap());
        db.upsert_video("abc123", &ch.id, "Test", None, "2026-04-05T00:00:00Z").unwrap();
        assert!(db.video_exists("abc123").unwrap());
    }

    #[test]
    fn upsert_video_updates_title_and_last_seen_on_conflict() {
        let db = test_db();
        let ch = insert_test_channel(&db);
        db.upsert_video("abc123", &ch.id, "Original Title", None, "2026-04-05T00:00:00Z").unwrap();
        // Mark as downloaded so we can assert it's preserved
        db.set_video_downloaded("abc123", "2026-04-05T06:00:00Z", "/tmp/test.mp4").unwrap();
        // Upsert again with updated title and last_seen_at
        db.upsert_video("abc123", &ch.id, "Updated Title", None, "2026-04-05T12:00:00Z").unwrap();
        let page = db.list_videos_for_channel(&ch.id, VideoFilter::Downloaded, false, None, 50, 0, None).unwrap();
        let videos = page.videos;
        assert_eq!(videos.len(), 1);
        assert_eq!(videos[0].title, "Updated Title");
        assert_eq!(videos[0].last_seen_at, "2026-04-05T12:00:00Z");
        // downloaded_at must not be overwritten by the upsert
        assert_eq!(videos[0].downloaded_at.as_deref(), Some("2026-04-05T06:00:00Z"));
    }

    #[test]
    fn fts_table_exists() {
        let db = test_db();
        let conn = db.conn.lock().unwrap();
        // FTS5 tables appear in sqlite_master as a table
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='videos_fts'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn fts_indexes_on_insert() {
        let db = test_db();
        let ch = insert_test_channel(&db);
        db.upsert_video("fts1", &ch.id, "Rust Programming Tutorial", None, "2026-04-05T00:00:00Z").unwrap();
        // FTS search should find it by title keyword
        let conn = db.conn.lock().unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM videos_fts WHERE videos_fts MATCH 'Programming'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn get_video_returns_none_for_missing() {
        let db = test_db();
        assert!(db.get_video("nonexistent").unwrap().is_none());
    }

    #[test]
    fn set_video_description_and_get_video() {
        let db = test_db();
        let ch = insert_test_channel(&db);
        db.upsert_video("vid1", &ch.id, "My Video", None, "2026-04-05T00:00:00Z").unwrap();
        db.set_video_description("vid1", "A great description").unwrap();
        let v = db.get_video("vid1").unwrap().unwrap();
        assert_eq!(v.description.as_deref(), Some("A great description"));
        assert_eq!(v.title, "My Video");
    }

    #[test]
    fn list_videos_search_filters_by_title() {
        let db = test_db();
        let ch = insert_test_channel(&db);
        db.upsert_video("v1", &ch.id, "Rust Programming", None, "2026-04-05T00:00:00Z").unwrap();
        db.upsert_video("v2", &ch.id, "Python Cooking", None, "2026-04-05T00:00:00Z").unwrap();
        let page = db.list_videos_for_channel(&ch.id, VideoFilter::All, false, Some("Rust"), 50, 0, None).unwrap();
        assert_eq!(page.videos.len(), 1);
        assert_eq!(page.videos[0].youtube_id, "v1");
        assert!(!page.has_more);
    }

    #[test]
    fn list_videos_pagination_works() {
        let db = test_db();
        let ch = insert_test_channel(&db);
        for i in 0..5 {
            db.upsert_video(&format!("v{i}"), &ch.id, &format!("Video {i}"), None, "2026-04-05T00:00:00Z").unwrap();
        }
        let page1 = db.list_videos_for_channel(&ch.id, VideoFilter::All, false, None, 3, 0, None).unwrap();
        assert_eq!(page1.videos.len(), 3);
        assert!(page1.has_more);

        let page2 = db.list_videos_for_channel(&ch.id, VideoFilter::All, false, None, 3, 3, None).unwrap();
        assert_eq!(page2.videos.len(), 2);
        assert!(!page2.has_more);
    }

    // ── Profile tests ──────────────────────────────────────────────────────────

    #[test]
    fn create_and_list_profiles() {
        let db = test_db();
        let p = db.create_profile("Alice", None, false).unwrap();
        assert_eq!(p.name, "Alice");
        assert!(!p.is_admin_profile);
        assert!(p.linked_email.is_none());

        let list = db.list_profiles(false).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, p.id);
    }

    #[test]
    fn admin_profiles_hidden_from_public_list() {
        let db = test_db();
        db.create_profile("admin@example.com", Some("admin@example.com"), true).unwrap();
        db.create_profile("Bob", None, false).unwrap();

        let public = db.list_profiles(false).unwrap();
        assert_eq!(public.len(), 1);
        assert_eq!(public[0].name, "Bob");

        let all = db.list_profiles(true).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn get_profile_by_email() {
        let db = test_db();
        db.create_profile("admin@example.com", Some("admin@example.com"), true).unwrap();
        let found = db.get_profile_by_email("admin@example.com").unwrap().unwrap();
        assert_eq!(found.linked_email.as_deref(), Some("admin@example.com"));
        assert!(found.is_admin_profile);

        assert!(db.get_profile_by_email("other@example.com").unwrap().is_none());
    }

    #[test]
    fn delete_profile_removes_it() {
        let db = test_db();
        let p = db.create_profile("Carol", None, false).unwrap();
        db.delete_profile(p.id).unwrap();
        assert!(db.get_profile(p.id).unwrap().is_none());
        assert_eq!(db.list_profiles(false).unwrap().len(), 0);
    }

    #[test]
    fn subscribe_and_list_channels() {
        let db = test_db();
        let p = db.create_profile("Dave", None, false).unwrap();
        let ch = db.insert_channel("https://youtube.com/@test", "Test").unwrap();

        db.subscribe_channel(p.id, &ch.id).unwrap();
        let ids = db.list_profile_channel_ids(p.id).unwrap();
        assert_eq!(ids, vec![ch.id.clone()]);

        db.unsubscribe_channel(p.id, &ch.id).unwrap();
        assert!(db.list_profile_channel_ids(p.id).unwrap().is_empty());
    }

    #[test]
    fn list_channels_for_profile_filters_subscriptions() {
        let db = test_db();
        let p = db.create_profile("Eve", None, false).unwrap();
        let ch1 = db.insert_channel("https://youtube.com/@A", "A").unwrap();
        let ch2 = db.insert_channel("https://youtube.com/@B", "B").unwrap();

        // Admin (None) sees all
        assert_eq!(db.list_channels_for_profile(None).unwrap().len(), 2);

        // Profile sees only subscribed channels
        db.subscribe_channel(p.id, &ch1.id).unwrap();
        let profile_channels = db.list_channels_for_profile(Some(p.id)).unwrap();
        assert_eq!(profile_channels.len(), 1);
        assert_eq!(profile_channels[0].id, ch1.id);

        let _ = ch2; // suppress unused warning
    }

    #[test]
    fn profile_video_ignore_and_unignore() {
        let db = test_db();
        let p = db.create_profile("Frank", None, false).unwrap();
        let ch = insert_test_channel(&db);
        db.upsert_video("v1", &ch.id, "Video", None, "2026-04-05T00:00:00Z").unwrap();

        db.ignore_video_for_profile(p.id, "v1").unwrap();

        // With profile: video is hidden
        let page = db.list_videos_for_channel(&ch.id, VideoFilter::All, false, None, 50, 0, Some(p.id)).unwrap();
        assert_eq!(page.videos.len(), 0);

        // Without profile (admin): video still visible
        let page_admin = db.list_videos_for_channel(&ch.id, VideoFilter::All, false, None, 50, 0, None).unwrap();
        assert_eq!(page_admin.videos.len(), 1);

        db.unignore_video_for_profile(p.id, "v1").unwrap();
        let page_after = db.list_videos_for_channel(&ch.id, VideoFilter::All, false, None, 50, 0, Some(p.id)).unwrap();
        assert_eq!(page_after.videos.len(), 1);
    }

    #[test]
    fn profile_ignored_status_reflected_in_get_video() {
        let db = test_db();
        let p = db.create_profile("Grace", None, false).unwrap();
        let ch = insert_test_channel(&db);
        db.upsert_video("v1", &ch.id, "Video", None, "2026-04-05T00:00:00Z").unwrap();

        db.ignore_video_for_profile(p.id, "v1").unwrap();

        let v_profile = db.get_video_for_profile("v1", Some(p.id)).unwrap().unwrap();
        assert_eq!(v_profile.status, VideoStatus::Ignored);

        let v_admin = db.get_video("v1").unwrap().unwrap();
        assert_eq!(v_admin.status, VideoStatus::New);
    }

    #[test]
    fn delete_profile_cascades_ignores_and_subscriptions() {
        let db = test_db();
        let p = db.create_profile("Henry", None, false).unwrap();
        let ch = insert_test_channel(&db);
        db.upsert_video("v1", &ch.id, "Video", None, "2026-04-05T00:00:00Z").unwrap();
        db.subscribe_channel(p.id, &ch.id).unwrap();
        db.ignore_video_for_profile(p.id, "v1").unwrap();

        db.delete_profile(p.id).unwrap();

        // Data cascaded cleanly
        let conn = db.conn.lock().unwrap();
        let ignore_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM profile_video_ignores WHERE profile_id=?1",
            rusqlite::params![p.id], |r| r.get(0),
        ).unwrap();
        assert_eq!(ignore_count, 0);
        let sub_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM profile_channels WHERE profile_id=?1",
            rusqlite::params![p.id], |r| r.get(0),
        ).unwrap();
        assert_eq!(sub_count, 0);
    }
}
