use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use yt_plex_common::models::{Job, JobStatus};

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
}
