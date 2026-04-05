# Channel Browser Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a channel browser so admins can maintain a list of approved YouTube channels and any local-network user can browse, filter, and download videos from those channels.

**Architecture:** Extend the existing Rust/Axum/SQLite backend with `channels` and `videos` tables, a `sync` module that uses yt-dlp flat-playlist to scrape video metadata, and a filesystem scanner that detects already-downloaded files by `[youtube_id]` suffix. Add SvelteKit pages for Browse, Queue, and Admin with tabbed navigation.

**Tech Stack:** Rust, Axum 0.8, rusqlite, tokio, yt-dlp (CLI), walkdir (new dep), SvelteKit 5 (Svelte runes), TypeScript.

---

## File Map

| File | Change |
|---|---|
| `crates/common/src/config.rs` | Add `SyncConfig`; add `sync` field to `Config` |
| `crates/common/src/models.rs` | Add `Channel`, `Video`, `VideoStatus`, `VideoFilter`; add `youtube_id` to `WsMessage` |
| `crates/server/Cargo.toml` | Add `walkdir = "2"` dependency |
| `crates/server/src/template.rs` | Add `id` parameter to `render()` |
| `crates/server/src/db.rs` | Add schema for `channels`/`videos`; add all channel and video DB operations |
| `crates/server/src/sync.rs` | **Create** — yt-dlp flat-playlist runner, line parser, filesystem scanner |
| `crates/server/src/worker.rs` | Call `db.set_video_downloaded()` on success; set `youtube_id` on `WsMessage` |
| `crates/server/src/lib.rs` | Add `pub mod sync`; spawn sync loop in `create_app_state`; add new routes to router |
| `crates/server/src/routes/mod.rs` | Add `pub mod channels; pub mod videos;` |
| `crates/server/src/routes/channels.rs` | **Create** — channel CRUD + per-channel sync trigger |
| `crates/server/src/routes/videos.rs` | **Create** — ignore/unignore endpoints |
| `crates/server/src/routes/jobs.rs` | Accept `{ youtube_id }` in addition to `{ url }` |
| `web/src/lib/api.ts` | Add `Channel`, `Video`, `VideoStatus` types; add all new API wrappers |
| `web/src/lib/ws.ts` | Add `youtube_id` to `WsMessage` interface |
| `web/src/routes/+layout.svelte` | Replace nav with tabs: Browse / Queue / Admin (admin-only) |
| `web/src/routes/+page.svelte` | Redirect to `/browse` |
| `web/src/routes/queue/+page.svelte` | **Create** — job list (moved from `/`) |
| `web/src/routes/browse/+page.svelte` | **Create** — channel grid |
| `web/src/routes/browse/[channelId]/+page.svelte` | **Create** — video grid with filters, Download, Ignore |
| `web/src/routes/admin/+page.svelte` | **Create** — settings + channel management + URL submission |
| `web/src/routes/settings/+page.svelte` | Replace with redirect to `/admin` |

---

## Task 1: SyncConfig + {id} template variable

**Files:**
- Modify: `crates/common/src/config.rs`
- Modify: `crates/server/src/template.rs`
- Modify: `crates/server/src/worker.rs` (update `template::render` call site)

- [ ] **Step 1: Write failing tests**

In `crates/common/src/config.rs`, add to the existing `#[cfg(test)]` block:

```rust
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
```

In `crates/server/src/template.rs`, update the existing `renders_all_variables` test and add a new one:

```rust
#[test]
fn renders_all_variables() {
    let result = render(
        "{channel}/{date} - {title} [{id}].{ext}",
        "MyChan",
        "2026-04-04",
        "My Video",
        "mp4",
        "dQw4w9WgXcQ",
    );
    assert_eq!(result, "MyChan/2026-04-04 - My Video [dQw4w9WgXcQ].mp4");
}

#[test]
fn renders_without_id_placeholder() {
    let result = render(
        "{channel}/{date} - {title}.{ext}",
        "Chan",
        "2026-04-04",
        "Vid",
        "mp4",
        "abc123",
    );
    assert_eq!(result, "Chan/2026-04-04 - Vid.mp4");
}
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cd /home/jamiet/code/yt-plex
cargo test -p yt-plex-common parses_config_with_sync 2>&1 | tail -5
cargo test -p yt-plex-server renders_all_variables 2>&1 | tail -5
```

Expected: compile errors — `SyncConfig` not defined, `render` has wrong arity.

- [ ] **Step 3: Add SyncConfig to config.rs**

In `crates/common/src/config.rs`, add after `OutputConfig`:

```rust
fn default_interval_hours() -> u64 { 6 }
fn default_playlist_items() -> usize { 50 }

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
```

Add `sync` field to `Config`:

```rust
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
```

- [ ] **Step 4: Update template::render to accept id**

Replace the full contents of `crates/server/src/template.rs`:

```rust
/// Render a path template with the given variables.
/// Sanitises channel and title to remove `/` and `\` to avoid path traversal.
pub fn render(template: &str, channel: &str, date: &str, title: &str, ext: &str, id: &str) -> String {
    let channel = sanitise(channel);
    let title = sanitise(title);
    template
        .replace("{channel}", &channel)
        .replace("{date}", date)
        .replace("{title}", &title)
        .replace("{ext}", ext)
        .replace("{id}", id)
}

fn sanitise(s: &str) -> String {
    s.replace('/', "_").replace('\\', "_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_all_variables() {
        let result = render(
            "{channel}/{date} - {title} [{id}].{ext}",
            "MyChan",
            "2026-04-04",
            "My Video",
            "mp4",
            "dQw4w9WgXcQ",
        );
        assert_eq!(result, "MyChan/2026-04-04 - My Video [dQw4w9WgXcQ].mp4");
    }

    #[test]
    fn renders_without_id_placeholder() {
        let result = render(
            "{channel}/{date} - {title}.{ext}",
            "Chan",
            "2026-04-04",
            "Vid",
            "mp4",
            "abc123",
        );
        assert_eq!(result, "Chan/2026-04-04 - Vid.mp4");
    }

    #[test]
    fn sanitises_path_separators_in_title() {
        let result = render("{title}.{ext}", "Chan", "2026-04-04", "foo/bar", "mp4", "id1");
        assert_eq!(result, "foo_bar.mp4");
    }
}
```

- [ ] **Step 5: Update the template::render call site in worker.rs**

In `crates/server/src/worker.rs`, find the line:
```rust
let rel_path = template::render(&path_template, &meta.channel, &date, &meta.title, &meta.ext);
```

Replace with:
```rust
let rel_path = template::render(&path_template, &meta.channel, &date, &meta.title, &meta.ext, &meta.id);
```

- [ ] **Step 6: Run tests**

```bash
cd /home/jamiet/code/yt-plex && cargo test 2>&1 | grep -E "^test result|FAILED"
```

Expected: all tests pass.

- [ ] **Step 7: Commit**

```bash
cd /home/jamiet/code/yt-plex
git add crates/common/src/config.rs crates/server/src/template.rs crates/server/src/worker.rs
git commit -m "feat: add SyncConfig and {id} template variable"
```

---

## Task 2: Channel and Video models + WsMessage youtube_id

**Files:**
- Modify: `crates/common/src/models.rs`

- [ ] **Step 1: Write failing tests**

Add to the `#[cfg(test)] mod tests` block in `crates/common/src/models.rs`:

```rust
#[test]
fn video_status_serialises() {
    let s = serde_json::to_string(&VideoStatus::New).unwrap();
    assert_eq!(s, "\"new\"");
    let s2 = serde_json::to_string(&VideoStatus::InProgress).unwrap();
    assert_eq!(s2, "\"in_progress\"");
}

#[test]
fn ws_message_includes_youtube_id_when_set() {
    let msg = WsMessage {
        job_id: "j1".into(),
        status: JobStatus::Done,
        channel_name: None,
        title: None,
        error: None,
        progress: None,
        youtube_id: Some("abc123".into()),
    };
    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("\"youtube_id\":\"abc123\""));
}

#[test]
fn ws_message_omits_youtube_id_when_none() {
    let msg = WsMessage {
        job_id: "j1".into(),
        status: JobStatus::Done,
        channel_name: None,
        title: None,
        error: None,
        progress: None,
        youtube_id: None,
    };
    let json = serde_json::to_string(&msg).unwrap();
    assert!(!json.contains("youtube_id"));
}
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cd /home/jamiet/code/yt-plex && cargo test -p yt-plex-common 2>&1 | tail -10
```

Expected: compile errors — `VideoStatus`, `Channel`, `Video` not defined; `WsMessage` missing `youtube_id`.

- [ ] **Step 3: Add models**

Replace the full contents of `crates/common/src/models.rs`:

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Downloading,
    Copying,
    Done,
    Failed,
}

impl JobStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Downloading => "downloading",
            Self::Copying => "copying",
            Self::Done => "done",
            Self::Failed => "failed",
        }
    }
}

impl FromStr for JobStatus {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "queued" => Ok(Self::Queued),
            "downloading" => Ok(Self::Downloading),
            "copying" => Ok(Self::Copying),
            "done" => Ok(Self::Done),
            "failed" => Ok(Self::Failed),
            other => Err(anyhow::anyhow!("unknown job status: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub url: String,
    pub status: JobStatus,
    pub channel_name: Option<String>,
    pub title: Option<String>,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Sent over WebSocket to all connected clients on job status change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsMessage {
    pub job_id: String,
    pub status: JobStatus,
    pub channel_name: Option<String>,
    pub title: Option<String>,
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub youtube_id: Option<String>,
}

impl WsMessage {
    pub fn from_job(job: &Job) -> Self {
        Self {
            job_id: job.id.clone(),
            status: job.status.clone(),
            channel_name: job.channel_name.clone(),
            title: job.title.clone(),
            error: job.error.clone(),
            progress: None,
            youtube_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: String,
    pub youtube_channel_url: String,
    pub name: String,
    pub last_synced_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VideoStatus {
    New,
    InProgress,
    Downloaded,
    Ignored,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Video {
    pub youtube_id: String,
    pub channel_id: String,
    pub title: String,
    pub published_at: Option<String>,
    pub downloaded_at: Option<String>,
    pub last_seen_at: String,
    pub ignored_at: Option<String>,
    pub status: VideoStatus,
}

/// Filter parameter for list_videos_for_channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VideoFilter {
    New,
    Downloaded,
    All,
}

impl VideoFilter {
    pub fn from_str(s: &str) -> Self {
        match s {
            "downloaded" => Self::Downloaded,
            "all" => Self::All,
            _ => Self::New,
        }
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::New => "new",
            Self::Downloaded => "downloaded",
            Self::All => "all",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_status_roundtrips_as_str() {
        assert_eq!(JobStatus::Queued.as_str(), "queued");
        assert_eq!(
            "downloading".parse::<JobStatus>().unwrap(),
            JobStatus::Downloading
        );
        assert!("bogus".parse::<JobStatus>().is_err());
    }

    #[test]
    fn ws_message_serialises() {
        let msg = WsMessage {
            job_id: "abc".into(),
            status: JobStatus::Done,
            channel_name: Some("Chan".into()),
            title: Some("Vid".into()),
            error: None,
            progress: None,
            youtube_id: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"status\":\"done\""));
        assert!(!json.contains("progress"));
        assert!(!json.contains("youtube_id"));

        let msg_with_progress = WsMessage {
            job_id: "abc".into(),
            status: JobStatus::Downloading,
            channel_name: None,
            title: None,
            error: None,
            progress: Some(42.5),
            youtube_id: None,
        };
        let json2 = serde_json::to_string(&msg_with_progress).unwrap();
        assert!(json2.contains("\"progress\":42.5"));
    }

    #[test]
    fn video_status_serialises() {
        let s = serde_json::to_string(&VideoStatus::New).unwrap();
        assert_eq!(s, "\"new\"");
        let s2 = serde_json::to_string(&VideoStatus::InProgress).unwrap();
        assert_eq!(s2, "\"in_progress\"");
    }

    #[test]
    fn ws_message_includes_youtube_id_when_set() {
        let msg = WsMessage {
            job_id: "j1".into(),
            status: JobStatus::Done,
            channel_name: None,
            title: None,
            error: None,
            progress: None,
            youtube_id: Some("abc123".into()),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"youtube_id\":\"abc123\""));
    }

    #[test]
    fn ws_message_omits_youtube_id_when_none() {
        let msg = WsMessage {
            job_id: "j1".into(),
            status: JobStatus::Done,
            channel_name: None,
            title: None,
            error: None,
            progress: None,
            youtube_id: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(!json.contains("youtube_id"));
    }
}
```

- [ ] **Step 4: Fix WsMessage construction in worker.rs**

The worker builds `WsMessage` structs inline (not just via `from_job`). Find all struct literal constructions of `WsMessage` in `crates/server/src/worker.rs` and add `youtube_id: None` to each one. They look like:

```rust
hub.broadcast(&yt_plex_common::models::WsMessage {
    job_id: job.id.clone(),
    status: JobStatus::Downloading,
    channel_name: None,
    title: None,
    error: None,
    progress: Some(pct),
    youtube_id: None,  // add this line
});
```

- [ ] **Step 5: Run all tests**

```bash
cd /home/jamiet/code/yt-plex && cargo test 2>&1 | grep -E "^test result|FAILED"
```

Expected: all pass.

- [ ] **Step 6: Commit**

```bash
cd /home/jamiet/code/yt-plex
git add crates/common/src/models.rs crates/server/src/worker.rs
git commit -m "feat: add Channel/Video/VideoStatus models and youtube_id to WsMessage"
```

---

## Task 3: DB schema — channels and videos tables

**Files:**
- Modify: `crates/server/src/db.rs`

- [ ] **Step 1: Write failing tests**

Add to the `#[cfg(test)] mod tests` block in `crates/server/src/db.rs`:

```rust
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
```

- [ ] **Step 2: Run to confirm they fail**

```bash
cd /home/jamiet/code/yt-plex && cargo test -p yt-plex-server channels_table_exists 2>&1 | tail -5
```

Expected: test fails — table `channels` does not exist.

- [ ] **Step 3: Add schema**

In `crates/server/src/db.rs`, append to the `SCHEMA` constant (before the closing `"`):

```rust
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
```

- [ ] **Step 4: Run tests**

```bash
cd /home/jamiet/code/yt-plex && cargo test -p yt-plex-server 2>&1 | grep -E "^test result|FAILED"
```

Expected: all pass (channels_table_exists and videos_table_exists now pass).

- [ ] **Step 5: Commit**

```bash
cd /home/jamiet/code/yt-plex
git add crates/server/src/db.rs
git commit -m "feat: add channels and videos tables to schema"
```

---

## Task 4: DB operations — channels

**Files:**
- Modify: `crates/server/src/db.rs`

Add `use yt_plex_common::models::Channel;` to the imports at the top of `db.rs`.

- [ ] **Step 1: Write failing tests**

Add to `#[cfg(test)] mod tests` in `db.rs`:

```rust
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
```

- [ ] **Step 2: Run to confirm they fail**

```bash
cd /home/jamiet/code/yt-plex && cargo test -p yt-plex-server insert_and_list_channels 2>&1 | tail -5
```

Expected: compile error — methods not defined.

- [ ] **Step 3: Implement channel DB operations**

Add to the `impl Db` block in `crates/server/src/db.rs`:

```rust
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
```

- [ ] **Step 4: Run tests**

```bash
cd /home/jamiet/code/yt-plex && cargo test -p yt-plex-server 2>&1 | grep -E "^test result|FAILED"
```

Expected: all pass.

- [ ] **Step 5: Commit**

```bash
cd /home/jamiet/code/yt-plex
git add crates/server/src/db.rs
git commit -m "feat: add channel DB operations"
```

---

## Task 5: DB operations — videos

**Files:**
- Modify: `crates/server/src/db.rs`

Add `use yt_plex_common::models::{Channel, Video, VideoFilter, VideoStatus};` to imports.

- [ ] **Step 1: Write failing tests**

Add to `#[cfg(test)] mod tests` in `db.rs`:

```rust
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
```

- [ ] **Step 2: Run to confirm they fail**

```bash
cd /home/jamiet/code/yt-plex && cargo test -p yt-plex-server upsert_and_list_videos 2>&1 | tail -5
```

Expected: compile error — methods not defined.

- [ ] **Step 3: Implement video DB operations**

Add to the `impl Db` block in `crates/server/src/db.rs`:

```rust
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
    // Build the WHERE clause for the filter.
    // status_cond uses only hardcoded strings; channel_id is bound via ?1.
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
```

- [ ] **Step 4: Run tests**

```bash
cd /home/jamiet/code/yt-plex && cargo test -p yt-plex-server 2>&1 | grep -E "^test result|FAILED"
```

Expected: all pass.

- [ ] **Step 5: Commit**

```bash
cd /home/jamiet/code/yt-plex
git add crates/server/src/db.rs
git commit -m "feat: add video DB operations (upsert, filter, ignore)"
```

---

## Task 6: Sync module

**Files:**
- Modify: `crates/server/Cargo.toml`
- Create: `crates/server/src/sync.rs`
- Modify: `crates/server/src/lib.rs` (add `pub mod sync`)

- [ ] **Step 1: Add walkdir dependency**

In `crates/server/Cargo.toml`, add to `[dependencies]`:

```toml
walkdir = "2"
```

- [ ] **Step 2: Write failing tests**

Create `crates/server/src/sync.rs` with just the test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn parse_flat_playlist_line_happy_path() {
        let entry = parse_flat_playlist_line("dQw4w9WgXcQ\tNever Gonna Give You Up\t19870727");
        assert_eq!(
            entry,
            Some(FlatPlaylistEntry {
                youtube_id: "dQw4w9WgXcQ".into(),
                title: "Never Gonna Give You Up".into(),
                published_at: Some("1987-07-27".into()),
            })
        );
    }

    #[test]
    fn parse_flat_playlist_line_missing_date() {
        let entry = parse_flat_playlist_line("abc123\tSome Video\tNA");
        assert_eq!(
            entry,
            Some(FlatPlaylistEntry {
                youtube_id: "abc123".into(),
                title: "Some Video".into(),
                published_at: None,
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
}
```

- [ ] **Step 3: Run tests to confirm they fail**

```bash
cd /home/jamiet/code/yt-plex && cargo test -p yt-plex-server parse_flat_playlist 2>&1 | tail -5
```

Expected: compile error — `parse_flat_playlist_line` not defined.

- [ ] **Step 4: Implement sync.rs**

Replace `crates/server/src/sync.rs` with the full implementation:

```rust
use anyhow::{Context, Result};
use chrono::Utc;
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
}

/// Parse one line of yt-dlp --flat-playlist --print "%(id)s\t%(title)s\t%(upload_date)s" output.
pub fn parse_flat_playlist_line(line: &str) -> Option<FlatPlaylistEntry> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    let mut parts = line.splitn(3, '\t');
    let youtube_id = parts.next()?.trim().to_string();
    if youtube_id.is_empty() {
        return None;
    }
    let title = parts.next().unwrap_or("").trim().to_string();
    let date_raw = parts.next().unwrap_or("").trim();
    // yt-dlp upload_date format is YYYYMMDD; convert to YYYY-MM-DD
    let published_at = parse_upload_date(date_raw);
    Some(FlatPlaylistEntry { youtube_id, title, published_at })
}

fn parse_upload_date(s: &str) -> Option<String> {
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
        "%(id)s\t%(title)s\t%(upload_date)s".to_string(),
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

    while let Ok(Some(line)) = lines.next_line().await {
        if let Some(entry) = parse_flat_playlist_line(&line) {
            db.upsert_video(
                &entry.youtube_id,
                channel_id,
                &entry.title,
                entry.published_at.as_deref(),
                &now,
            )
            .context("upserting video")?;
            count += 1;
        }
    }

    child.wait().await.context("waiting for yt-dlp")?;
    db.set_channel_synced(channel_id, &now)?;
    info!("synced {count} videos for {channel_url}");
    Ok(())
}

/// Walk base_path and set downloaded_at on any video whose youtube_id appears
/// in a filename as `[youtube_id]`.
pub fn scan_filesystem(base_path: &str, db: &Db) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    let mut found = 0usize;
    for entry in walkdir::WalkDir::new(base_path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        if let Some(youtube_id) = extract_youtube_id_from_path(entry.path()) {
            if db.video_exists(&youtube_id)? {
                db.set_video_downloaded(&youtube_id, &now)?;
                found += 1;
            }
        }
    }
    info!("filesystem scan: {found} videos marked as downloaded");
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
        let entry = parse_flat_playlist_line("dQw4w9WgXcQ\tNever Gonna Give You Up\t19870727");
        assert_eq!(
            entry,
            Some(FlatPlaylistEntry {
                youtube_id: "dQw4w9WgXcQ".into(),
                title: "Never Gonna Give You Up".into(),
                published_at: Some("1987-07-27".into()),
            })
        );
    }

    #[test]
    fn parse_flat_playlist_line_missing_date() {
        let entry = parse_flat_playlist_line("abc123\tSome Video\tNA");
        assert_eq!(
            entry,
            Some(FlatPlaylistEntry {
                youtube_id: "abc123".into(),
                title: "Some Video".into(),
                published_at: None,
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
        let path = Path::new("/plex/Chan/Video [playlist] [abc123def45].mp4");
        assert_eq!(
            extract_youtube_id_from_path(path),
            Some("abc123def45".into())
        );
    }
}
```

- [ ] **Step 5: Add `pub mod sync` to lib.rs**

In `crates/server/src/lib.rs`, add to the module declarations at the top:

```rust
pub mod sync;
```

- [ ] **Step 6: Run tests**

```bash
cd /home/jamiet/code/yt-plex && cargo test -p yt-plex-server 2>&1 | grep -E "^test result|FAILED"
```

Expected: all pass including new sync tests.

- [ ] **Step 7: Commit**

```bash
cd /home/jamiet/code/yt-plex
git add crates/server/Cargo.toml crates/server/src/sync.rs crates/server/src/lib.rs
git commit -m "feat: add sync module (yt-dlp flat-playlist, filesystem scan)"
```

---

## Task 7: Background sync loop + worker youtube_id

**Files:**
- Modify: `crates/server/src/lib.rs`
- Modify: `crates/server/src/worker.rs`

- [ ] **Step 1: Spawn sync loop in create_app_state**

In `crates/server/src/lib.rs`, find this block near the end of `create_app_state`:

```rust
    let ws_hub = WsHub::new();
    Ok(AppState {
        db,
        config: Arc::new(std::sync::RwLock::new(config)),
        config_path,
        ws_hub,
        oauth_states: Arc::new(Mutex::new(HashMap::new())),
        http_client: reqwest::Client::new(),
    })
```

Replace it with:

```rust
    let ws_hub = WsHub::new();
    let config_arc = Arc::new(std::sync::RwLock::new(config));

    // Spawn background channel sync loop
    {
        let sync_db = Arc::clone(&db);
        let sync_config = Arc::clone(&config_arc);
        tokio::spawn(async move {
            crate::sync::run_sync_loop(sync_db, sync_config).await;
        });
    }

    Ok(AppState {
        db,
        config: config_arc,
        config_path,
        ws_hub,
        oauth_states: Arc::new(Mutex::new(HashMap::new())),
        http_client: reqwest::Client::new(),
    })
```

- [ ] **Step 2: Update worker.rs to set youtube_id on WsMessage and call set_video_downloaded**

In `crates/server/src/worker.rs`, after the successful download and file move (just before `db.update_job(&job.id, JobStatus::Done, ...)`), add:

```rust
    // Mark video as downloaded in the videos table (if it was tracked)
    if db.video_exists(&meta.id).unwrap_or(false) {
        let now_str = Utc::now().to_rfc3339();
        if let Err(e) = db.set_video_downloaded(&meta.id, &now_str) {
            warn!("set_video_downloaded failed for {}: {e:#}", meta.id);
        }
    }
```

Also, in `worker.rs`, store `youtube_id` on the job when we have it. In `tick()`, the job is fetched from DB. The `insert_job_with_youtube_id` route (Task 8) will store the youtube_id in the job URL. But we can extract it from `meta.id` after yt-dlp runs.

Add a field to `Job` is out of scope. Instead, set `youtube_id` on the final Done WsMessage:

```rust
    db.update_job(&job.id, JobStatus::Done, None, None, None)?;
    let updated = db.get_job(&job.id)?.unwrap();
    let mut done_msg = yt_plex_common::models::WsMessage::from_job(&updated);
    done_msg.youtube_id = Some(meta.id.clone());
    hub.broadcast(&done_msg);
```

Replace the existing `done` broadcast block with the above.

- [ ] **Step 3: Build**

```bash
cd /home/jamiet/code/yt-plex && cargo build -p yt-plex-server 2>&1 | grep "^error" | head -20
```

Expected: clean build.

- [ ] **Step 4: Run all tests**

```bash
cd /home/jamiet/code/yt-plex && cargo test 2>&1 | grep -E "^test result|FAILED"
```

Expected: all pass.

- [ ] **Step 5: Commit**

```bash
cd /home/jamiet/code/yt-plex
git add crates/server/src/lib.rs crates/server/src/worker.rs
git commit -m "feat: spawn background sync loop; set youtube_id on done WsMessage"
```

---

## Task 8: API routes — channels

**Files:**
- Create: `crates/server/src/routes/channels.rs`
- Modify: `crates/server/src/routes/mod.rs`
- Modify: `crates/server/src/lib.rs`

- [ ] **Step 1: Create routes/channels.rs**

Create `crates/server/src/routes/channels.rs`:

```rust
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
use tracing::error;

use crate::{routes::auth::SessionToken, sync, AppState};

fn is_admin(state: &AppState, token: Option<&str>) -> bool {
    token
        .and_then(|t| state.db.is_valid_session(t).ok())
        .unwrap_or(false)
}

pub async fn list_channels(State(state): State<AppState>) -> impl IntoResponse {
    match state.db.list_channels() {
        Ok(channels) => Json(channels).into_response(),
        Err(e) => {
            error!("list_channels: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Server error").into_response()
        }
    }
}

#[derive(Deserialize)]
pub struct AddChannelRequest {
    pub url: String,
    pub name: String,
}

pub async fn add_channel(
    State(state): State<AppState>,
    SessionToken(token): SessionToken,
    Json(body): Json<AddChannelRequest>,
) -> impl IntoResponse {
    if !is_admin(&state, token.as_deref()) {
        return (StatusCode::UNAUTHORIZED, "Admin required").into_response();
    }
    let channel = match state.db.insert_channel(&body.url, &body.name) {
        Ok(ch) => ch,
        Err(e) => {
            error!("insert_channel: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Server error").into_response();
        }
    };
    // Trigger first sync in background
    let db = Arc::clone(&state.db);
    let config = state.config.read().unwrap().clone();
    let ch_id = channel.id.clone();
    let ch_url = channel.youtube_channel_url.clone();
    tokio::spawn(async move {
        if let Err(e) = sync::sync_channel(&ch_id, &ch_url, &db, &config, true).await {
            error!("initial sync failed for {ch_url}: {e:#}");
        }
    });
    Json(channel).into_response()
}

pub async fn delete_channel(
    State(state): State<AppState>,
    SessionToken(token): SessionToken,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if !is_admin(&state, token.as_deref()) {
        return (StatusCode::UNAUTHORIZED, "Admin required").into_response();
    }
    match state.db.delete_channel(&id) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => {
            error!("delete_channel: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Server error").into_response()
        }
    }
}

pub async fn sync_channel(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let channel = match state.db.get_channel(&id) {
        Ok(Some(ch)) => ch,
        Ok(None) => return (StatusCode::NOT_FOUND, "Channel not found").into_response(),
        Err(e) => {
            error!("get_channel: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Server error").into_response();
        }
    };
    let db = Arc::clone(&state.db);
    let config = state.config.read().unwrap().clone();
    let ch_id = channel.id.clone();
    let ch_url = channel.youtube_channel_url.clone();
    let is_first = channel.last_synced_at.is_none();
    tokio::spawn(async move {
        if let Err(e) = sync::sync_channel(&ch_id, &ch_url, &db, &config, is_first).await {
            error!("manual sync failed for {ch_url}: {e:#}");
        }
    });
    StatusCode::ACCEPTED.into_response()
}

pub async fn list_channel_videos(
    State(state): State<AppState>,
    Path(id): Path<String>,
    axum::extract::Query(params): axum::extract::Query<VideoQueryParams>,
) -> impl IntoResponse {
    let filter = yt_plex_common::models::VideoFilter::from_str(
        params.filter.as_deref().unwrap_or("new"),
    );
    let show_ignored = params.show_ignored.unwrap_or(false);
    match state.db.list_videos_for_channel(&id, filter, show_ignored) {
        Ok(videos) => Json(videos).into_response(),
        Err(e) => {
            error!("list_videos_for_channel: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Server error").into_response()
        }
    }
}

#[derive(Deserialize)]
pub struct VideoQueryParams {
    pub filter: Option<String>,
    pub show_ignored: Option<bool>,
}
```

- [ ] **Step 2: Add to routes/mod.rs**

In `crates/server/src/routes/mod.rs`:

```rust
pub mod assets;
pub mod auth;
pub mod channels;
pub mod jobs;
pub mod settings;
pub mod videos;
```

- [ ] **Step 3: Wire routes into lib.rs router**

In `crates/server/src/lib.rs`, update `build_router` to add:

```rust
use axum::routing::delete;

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/api/auth/login", get(routes::auth::device_login))
        .route("/api/auth/poll", get(routes::auth::device_poll))
        .route("/api/auth/me", get(routes::auth::me))
        .route("/api/logout", post(routes::auth::logout))
        .route("/api/jobs", post(routes::jobs::submit_job))
        .route("/api/jobs", get(routes::jobs::list_jobs))
        .route("/api/settings", get(routes::settings::get_settings))
        .route("/api/settings", put(routes::settings::update_settings))
        .route("/api/channels", get(routes::channels::list_channels))
        .route("/api/channels", post(routes::channels::add_channel))
        .route("/api/channels/:id", delete(routes::channels::delete_channel))
        .route("/api/channels/:id/sync", post(routes::channels::sync_channel))
        .route("/api/channels/:id/videos", get(routes::channels::list_channel_videos))
        .route("/api/videos/:youtube_id/ignore", post(routes::videos::ignore_video))
        .route("/api/videos/:youtube_id/ignore", delete(routes::videos::unignore_video))
        .route("/ws", get(ws::ws_handler))
        .fallback(routes::assets::serve_asset)
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}
```

- [ ] **Step 4: Build**

```bash
cd /home/jamiet/code/yt-plex && cargo build -p yt-plex-server 2>&1 | grep "^error" | head -20
```

Fix any compile errors (likely `routes::videos` not yet created — create a stub in Step 5).

- [ ] **Step 5: Create stub routes/videos.rs**

Create `crates/server/src/routes/videos.rs` with stubs so lib.rs compiles:

```rust
use axum::{extract::{Path, State}, http::StatusCode, response::IntoResponse};
use chrono::Utc;
use tracing::error;
use crate::AppState;

pub async fn ignore_video(
    State(state): State<AppState>,
    Path(youtube_id): Path<String>,
) -> impl IntoResponse {
    let now = Utc::now().to_rfc3339();
    match state.db.ignore_video(&youtube_id, &now) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => {
            error!("ignore_video: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Server error").into_response()
        }
    }
}

pub async fn unignore_video(
    State(state): State<AppState>,
    Path(youtube_id): Path<String>,
) -> impl IntoResponse {
    match state.db.unignore_video(&youtube_id) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => {
            error!("unignore_video: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Server error").into_response()
        }
    }
}
```

- [ ] **Step 6: Build and test**

```bash
cd /home/jamiet/code/yt-plex && cargo build -p yt-plex-server 2>&1 | grep "^error" | head -20
cargo test 2>&1 | grep -E "^test result|FAILED"
```

Expected: clean build, all tests pass.

- [ ] **Step 7: Commit**

```bash
cd /home/jamiet/code/yt-plex
git add crates/server/src/routes/channels.rs crates/server/src/routes/videos.rs \
        crates/server/src/routes/mod.rs crates/server/src/lib.rs
git commit -m "feat: add channel and video API routes"
```

---

## Task 9: Modify POST /api/jobs to accept youtube_id

**Files:**
- Modify: `crates/server/src/routes/jobs.rs`

- [ ] **Step 1: Update submit_job to accept youtube_id**

Replace the full contents of `crates/server/src/routes/jobs.rs`:

```rust
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;

use crate::{routes::auth::SessionToken, AppState};

fn is_admin(state: &AppState, token: Option<&str>) -> bool {
    token
        .and_then(|t| state.db.is_valid_session(t).ok())
        .unwrap_or(false)
}

#[derive(Deserialize)]
pub struct SubmitJobRequest {
    pub url: Option<String>,
    pub youtube_id: Option<String>,
}

pub async fn submit_job(
    State(state): State<AppState>,
    SessionToken(token): SessionToken,
    Json(body): Json<SubmitJobRequest>,
) -> impl IntoResponse {
    let url = if let Some(youtube_id) = &body.youtube_id {
        // Any user may queue a download by youtube_id — but only if the video
        // exists in an approved channel.
        match state.db.video_exists(youtube_id) {
            Ok(true) => format!("https://www.youtube.com/watch?v={youtube_id}"),
            Ok(false) => {
                return (StatusCode::NOT_FOUND, "Video not in any approved channel").into_response()
            }
            Err(e) => {
                tracing::error!("video_exists: {e}");
                return (StatusCode::INTERNAL_SERVER_ERROR, "Server error").into_response();
            }
        }
    } else if let Some(url) = &body.url {
        // Arbitrary URL submission — admin only.
        if !is_admin(&state, token.as_deref()) {
            return (StatusCode::UNAUTHORIZED, "Admin required").into_response();
        }
        if !url.starts_with("https://www.youtube.com/")
            && !url.starts_with("https://youtu.be/")
        {
            return (StatusCode::BAD_REQUEST, "Only YouTube URLs are accepted").into_response();
        }
        url.clone()
    } else {
        return (StatusCode::BAD_REQUEST, "Provide either url or youtube_id").into_response();
    };

    match state.db.insert_job(&url) {
        Ok(job) => Json(job).into_response(),
        Err(e) => {
            tracing::error!("insert job: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Server error").into_response()
        }
    }
}

pub async fn list_jobs(State(state): State<AppState>) -> impl IntoResponse {
    match state.db.list_jobs() {
        Ok(jobs) => Json(jobs).into_response(),
        Err(e) => {
            tracing::error!("list jobs: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Server error").into_response()
        }
    }
}
```

- [ ] **Step 2: Build and test**

```bash
cd /home/jamiet/code/yt-plex && cargo build -p yt-plex-server 2>&1 | grep "^error" | head -10
cargo test 2>&1 | grep -E "^test result|FAILED"
```

Expected: clean build, all tests pass.

- [ ] **Step 3: Commit**

```bash
cd /home/jamiet/code/yt-plex
git add crates/server/src/routes/jobs.rs
git commit -m "feat: accept youtube_id in POST /api/jobs for anonymous users"
```

---

## Task 10: Frontend API types and WS update

**Files:**
- Modify: `web/src/lib/api.ts`
- Modify: `web/src/lib/ws.ts`

- [ ] **Step 1: Update api.ts**

Replace the `Job` interface and add new types and wrappers. Find the `export interface Job` block and the end of the file in `web/src/lib/api.ts`. Add after the existing exports:

```typescript
export type VideoStatus = 'new' | 'in_progress' | 'downloaded' | 'ignored';

export interface Channel {
    id: string;
    youtube_channel_url: string;
    name: string;
    last_synced_at: string | null;
}

export interface Video {
    youtube_id: string;
    channel_id: string;
    title: string;
    published_at: string | null;
    downloaded_at: string | null;
    last_seen_at: string;
    ignored_at: string | null;
    status: VideoStatus;
}

export async function listChannels(): Promise<Channel[]> {
    const res = await fetch('/api/channels');
    if (!res.ok) throw new Error(`listChannels failed: ${res.status}`);
    return res.json();
}

export async function addChannel(url: string, name: string): Promise<Channel> {
    const res = await fetch('/api/channels', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ url, name }),
    });
    if (!res.ok) {
        const text = await res.text();
        throw new Error(text || `addChannel failed: ${res.status}`);
    }
    return res.json();
}

export async function deleteChannel(id: string): Promise<void> {
    const res = await fetch(`/api/channels/${id}`, { method: 'DELETE' });
    if (!res.ok) throw new Error(`deleteChannel failed: ${res.status}`);
}

export async function syncChannel(id: string): Promise<void> {
    const res = await fetch(`/api/channels/${id}/sync`, { method: 'POST' });
    if (!res.ok) throw new Error(`syncChannel failed: ${res.status}`);
}

export async function listVideos(
    channelId: string,
    filter: 'new' | 'downloaded' | 'all' = 'new',
    showIgnored = false,
): Promise<Video[]> {
    const params = new URLSearchParams({ filter });
    if (showIgnored) params.set('show_ignored', 'true');
    const res = await fetch(`/api/channels/${channelId}/videos?${params}`);
    if (!res.ok) throw new Error(`listVideos failed: ${res.status}`);
    return res.json();
}

export async function ignoreVideo(youtubeId: string): Promise<void> {
    const res = await fetch(`/api/videos/${youtubeId}/ignore`, { method: 'POST' });
    if (!res.ok) throw new Error(`ignoreVideo failed: ${res.status}`);
}

export async function unignoreVideo(youtubeId: string): Promise<void> {
    const res = await fetch(`/api/videos/${youtubeId}/ignore`, { method: 'DELETE' });
    if (!res.ok) throw new Error(`unignoreVideo failed: ${res.status}`);
}

export async function submitJobByYoutubeId(youtubeId: string): Promise<Job> {
    const res = await fetch('/api/jobs', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ youtube_id: youtubeId }),
    });
    if (!res.ok) {
        const text = await res.text();
        throw new Error(text || `submitJob failed: ${res.status}`);
    }
    return res.json();
}
```

Also update the existing `submitJob` function to use `{ url }` explicitly (it already does, but confirm it's unchanged).

- [ ] **Step 2: Update ws.ts**

In `web/src/lib/ws.ts`, update `WsMessage`:

```typescript
export interface WsMessage {
    job_id: string;
    status: Job['status'];
    channel_name: string | null;
    title: string | null;
    error: string | null;
    progress?: number | null;
    youtube_id?: string | null;
}
```

- [ ] **Step 3: Build frontend**

```bash
cd /home/jamiet/code/yt-plex/web && pnpm build 2>&1 | tail -5
```

Expected: `✔ done`

- [ ] **Step 4: Commit**

```bash
cd /home/jamiet/code/yt-plex
git add web/src/lib/api.ts web/src/lib/ws.ts
git commit -m "feat: add Channel/Video API types and wrappers; add youtube_id to WsMessage"
```

---

## Task 11: Tab navigation + queue page

**Files:**
- Modify: `web/src/routes/+layout.svelte`
- Modify: `web/src/routes/+page.svelte`
- Create: `web/src/routes/queue/+page.svelte`

- [ ] **Step 1: Create /queue page**

Create `web/src/routes/queue/+page.svelte` — move the existing job list from `+page.svelte`:

```svelte
<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { listJobs, submitJob, type Job } from '$lib/api';
    import { createWsStore } from '$lib/ws';

    let jobs = $state<Job[]>([]);
    let url = $state('');
    let submitError = $state('');
    let submitting = $state(false);

    const ws = createWsStore();

    onMount(async () => {
        try { jobs = await listJobs(); } catch { /* ignore */ }
        ws.connect();
    });
    onDestroy(() => ws.disconnect());

    let unsubWs: (() => void) | undefined;
    onMount(() => { unsubWs = ws.subscribe(() => {}); });
    onDestroy(() => unsubWs?.());

    $effect(() => {
        const msg = $ws;
        if (msg) {
            jobs = jobs.map(j =>
                j.id === msg.job_id
                    ? {
                        ...j,
                        status: msg.status,
                        channel_name: msg.channel_name ?? j.channel_name,
                        title: msg.title ?? j.title,
                        error: msg.error,
                        progress: msg.progress ?? (msg.status !== 'downloading' ? null : j.progress),
                      }
                    : j
            );
        }
    });

    async function handleSubmit() {
        submitError = '';
        submitting = true;
        try {
            const job = await submitJob(url);
            jobs = [job, ...jobs];
            url = '';
        } catch (e: unknown) {
            submitError = e instanceof Error ? e.message : 'Submit failed';
        } finally {
            submitting = false;
        }
    }

    const statusColour: Record<Job['status'], string> = {
        queued: '#888',
        downloading: '#4af',
        copying: '#fa4',
        done: '#4c4',
        failed: '#f44',
    };
</script>

<main>
    <form onsubmit={(e) => { e.preventDefault(); handleSubmit(); }}>
        <input type="url" bind:value={url} placeholder="https://www.youtube.com/watch?v=…" disabled={submitting} />
        <button type="submit" disabled={submitting || !url}>Add URL (admin)</button>
    </form>
    {#if submitError}<p class="error">{submitError}</p>{/if}

    <table>
        <thead>
            <tr><th>Status</th><th>Channel</th><th>Title</th><th>URL</th><th>Added</th></tr>
        </thead>
        <tbody>
            {#each jobs as job (job.id)}
                <tr>
                    <td style="color:{statusColour[job.status]}">
                        {job.status}
                        {#if job.status === 'downloading' && job.progress != null}
                            <span class="progress">{job.progress.toFixed(0)}%</span>
                        {/if}
                    </td>
                    <td>{job.channel_name ?? '—'}</td>
                    <td>
                        {job.title ?? '—'}
                        {#if job.error}<span class="error" title={job.error}> ⚠</span>{/if}
                    </td>
                    <td><a href={job.url} target="_blank" rel="noreferrer">link</a></td>
                    <td>{new Date(job.created_at).toLocaleString()}</td>
                </tr>
            {/each}
        </tbody>
    </table>
</main>

<style>
    main { padding: 1rem; font-family: sans-serif; }
    form { display: flex; gap: 0.5rem; margin-bottom: 1rem; }
    input[type=url] { flex: 1; padding: 0.4rem; }
    table { width: 100%; border-collapse: collapse; }
    th, td { text-align: left; padding: 0.4rem 0.6rem; border-bottom: 1px solid #333; }
    .error { color: red; }
    .progress { font-size: 0.85em; opacity: 0.8; margin-left: 0.3em; }
</style>
```

- [ ] **Step 2: Update +page.svelte to redirect**

Replace `web/src/routes/+page.svelte` with:

```svelte
<script lang="ts">
    import { goto } from '$app/navigation';
    goto('/browse', { replaceState: true });
</script>
```

- [ ] **Step 3: Update +layout.svelte with tab navigation**

Replace `web/src/routes/+layout.svelte`:

```svelte
<script lang="ts">
    import { logout } from '$lib/api';
    import { page } from '$app/stores';
    import type { Snippet } from 'svelte';

    let { children }: { children: Snippet } = $props();
    let isAdmin = $state(false);

    $effect(() => {
        fetch('/api/auth/me').then(r => { isAdmin = r.ok; });
    });

    async function handleLogout() {
        await logout();
        isAdmin = false;
        window.location.href = '/login';
    }

    function isActive(prefix: string) {
        return $page.url.pathname.startsWith(prefix);
    }
</script>

{#if $page.url.pathname !== '/login'}
<nav>
    <a href="/browse" class:active={isActive('/browse')}>Browse</a>
    <a href="/queue" class:active={isActive('/queue')}>Queue</a>
    {#if isAdmin}
        <a href="/admin" class:active={isActive('/admin')}>Admin</a>
        <button class="logout" onclick={handleLogout}>Log out</button>
    {/if}
</nav>
{/if}

{@render children()}

<style>
    nav {
        display: flex;
        gap: 0;
        background: #111;
        border-bottom: 1px solid #333;
        padding: 0 1rem;
        align-items: center;
    }
    nav a {
        color: #888;
        text-decoration: none;
        padding: 0.65rem 1rem;
        font-size: 0.9rem;
        border-bottom: 2px solid transparent;
        margin-bottom: -1px;
    }
    nav a.active { color: #4af; border-bottom-color: #4af; }
    nav a:hover:not(.active) { color: #ccc; }
    .logout {
        margin-left: auto;
        cursor: pointer;
        background: none;
        border: 1px solid #555;
        color: #888;
        padding: 0.25rem 0.75rem;
        border-radius: 4px;
        font-size: 0.85rem;
    }
</style>
```

- [ ] **Step 4: Build**

```bash
cd /home/jamiet/code/yt-plex/web && pnpm build 2>&1 | tail -5
```

Expected: `✔ done`

- [ ] **Step 5: Commit**

```bash
cd /home/jamiet/code/yt-plex
git add web/src/routes/+layout.svelte web/src/routes/+page.svelte web/src/routes/queue/+page.svelte
git commit -m "feat: tab navigation + /queue page (moved from /)"
```

---

## Task 12: Frontend — /browse channel grid

**Files:**
- Create: `web/src/routes/browse/+page.svelte`

- [ ] **Step 1: Create browse page**

Create `web/src/routes/browse/+page.svelte`:

```svelte
<script lang="ts">
    import { onMount } from 'svelte';
    import { listChannels, type Channel } from '$lib/api';

    let channels = $state<Channel[]>([]);
    let error = $state('');

    onMount(async () => {
        try {
            channels = await listChannels();
        } catch (e: unknown) {
            error = e instanceof Error ? e.message : 'Failed to load channels';
        }
    });

    function timeAgo(isoString: string | null): string {
        if (!isoString) return 'never';
        const diff = Date.now() - new Date(isoString).getTime();
        const hours = Math.floor(diff / 3600000);
        if (hours < 1) return 'just now';
        if (hours < 24) return `${hours}h ago`;
        return `${Math.floor(hours / 24)}d ago`;
    }
</script>

<main>
    {#if error}
        <p class="error">{error}</p>
    {:else if channels.length === 0}
        <p class="empty">No channels configured yet. An admin can add channels from the Admin page.</p>
    {:else}
        <div class="grid">
            {#each channels as channel (channel.id)}
                <a href="/browse/{channel.id}" class="card">
                    <div class="card-name">{channel.name}</div>
                    <div class="card-meta">Synced {timeAgo(channel.last_synced_at)}</div>
                </a>
            {/each}
        </div>
    {/if}
</main>

<style>
    main { padding: 1rem; font-family: sans-serif; }
    .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 0.75rem; }
    .card {
        display: block;
        background: #1e1e2e;
        border: 1px solid #2a2a3a;
        border-radius: 8px;
        padding: 1rem;
        text-decoration: none;
        color: inherit;
        transition: border-color 0.15s;
    }
    .card:hover { border-color: #4af; }
    .card-name { font-weight: 600; color: #ddd; margin-bottom: 0.3rem; }
    .card-meta { font-size: 0.8rem; color: #666; }
    .empty { color: #888; font-style: italic; }
    .error { color: red; }
</style>
```

- [ ] **Step 2: Build**

```bash
cd /home/jamiet/code/yt-plex/web && pnpm build 2>&1 | tail -5
```

- [ ] **Step 3: Commit**

```bash
cd /home/jamiet/code/yt-plex
git add web/src/routes/browse/+page.svelte
git commit -m "feat: /browse channel grid page"
```

---

## Task 13: Frontend — /browse/[channelId] video grid

**Files:**
- Create: `web/src/routes/browse/[channelId]/+page.svelte`

- [ ] **Step 1: Create video grid page**

Create `web/src/routes/browse/[channelId]/+page.svelte`:

```svelte
<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { page } from '$app/stores';
    import {
        listChannels, listVideos, ignoreVideo, unignoreVideo,
        submitJobByYoutubeId, syncChannel, type Channel, type Video, type VideoStatus
    } from '$lib/api';
    import { createWsStore } from '$lib/ws';

    const channelId = $derived($page.params.channelId);

    let channel = $state<Channel | null>(null);
    let videos = $state<Video[]>([]);
    let filter = $state<'new' | 'downloaded' | 'all'>('new');
    let showIgnored = $state(false);
    let error = $state('');
    let syncing = $state(false);

    const ws = createWsStore();
    let unsubWs: (() => void) | undefined;

    async function loadVideos() {
        try {
            videos = await listVideos(channelId, filter, showIgnored);
        } catch (e: unknown) {
            error = e instanceof Error ? e.message : 'Failed to load videos';
        }
    }

    onMount(async () => {
        try {
            const channels = await listChannels();
            channel = channels.find(c => c.id === channelId) ?? null;
        } catch { /* ignore */ }
        await loadVideos();
        ws.connect();
        unsubWs = ws.subscribe(() => {});
    });

    onDestroy(() => { ws.disconnect(); unsubWs?.(); });

    // Apply real-time WS updates to video status
    $effect(() => {
        const msg = $ws;
        if (!msg?.youtube_id) return;
        videos = videos.map(v => {
            if (v.youtube_id !== msg.youtube_id) return v;
            const newStatus: VideoStatus =
                msg.status === 'done' ? 'downloaded'
                : (msg.status === 'queued' || msg.status === 'downloading' || msg.status === 'copying') ? 'in_progress'
                : v.status;
            return { ...v, status: newStatus };
        });
    });

    async function handleDownload(video: Video) {
        try {
            await submitJobByYoutubeId(video.youtube_id);
            // Optimistically update to in_progress
            videos = videos.map(v =>
                v.youtube_id === video.youtube_id ? { ...v, status: 'in_progress' as VideoStatus } : v
            );
        } catch (e: unknown) {
            alert(e instanceof Error ? e.message : 'Failed to queue download');
        }
    }

    async function handleIgnore(video: Video) {
        try {
            await ignoreVideo(video.youtube_id);
            videos = videos.map(v =>
                v.youtube_id === video.youtube_id ? { ...v, status: 'ignored' as VideoStatus, ignored_at: new Date().toISOString() } : v
            );
        } catch { /* ignore */ }
    }

    async function handleUnignore(video: Video) {
        try {
            await unignoreVideo(video.youtube_id);
            videos = videos.map(v =>
                v.youtube_id === video.youtube_id ? { ...v, status: 'new' as VideoStatus, ignored_at: null } : v
            );
        } catch { /* ignore */ }
    }

    async function handleSync() {
        syncing = true;
        try {
            await syncChannel(channelId);
            // Reload after a brief delay to let sync start
            setTimeout(loadVideos, 2000);
        } catch { /* ignore */ } finally {
            syncing = false;
        }
    }

    $effect(() => {
        // Reload when filter or showIgnored changes
        void filter;
        void showIgnored;
        loadVideos();
    });

    const statusLabel: Record<VideoStatus, string> = {
        new: 'NEW',
        in_progress: '↓',
        downloaded: '✓ ON PLEX',
        ignored: 'IGNORED',
    };
    const statusColour: Record<VideoStatus, string> = {
        new: '#4af',
        in_progress: '#fa4',
        downloaded: '#4c4',
        ignored: '#555',
    };
</script>

<main>
    <div class="header">
        <a href="/browse" class="back">← Channels</a>
        <span class="channel-name">{channel?.name ?? channelId}</span>
        <button class="refresh" onclick={handleSync} disabled={syncing}>
            {syncing ? 'Syncing…' : '↻ Refresh'}
        </button>
    </div>

    <div class="filters">
        <span class="label">Show:</span>
        {#each (['new', 'downloaded', 'all'] as const) as f}
            <button
                class="pill"
                class:active={filter === f}
                onclick={() => { filter = f; }}
            >{f}</button>
        {/each}
        <label class="toggle">
            <input type="checkbox" bind:checked={showIgnored} />
            Show ignored
        </label>
    </div>

    {#if error}<p class="error">{error}</p>{/if}

    <div class="grid">
        {#each videos as video (video.youtube_id)}
            <div class="card">
                <div class="thumb">
                    <img
                        src="https://img.youtube.com/vi/{video.youtube_id}/mqdefault.jpg"
                        alt={video.title}
                        loading="lazy"
                    />
                    <span class="badge" style="background:{statusColour[video.status]}">
                        {statusLabel[video.status]}
                    </span>
                </div>
                <div class="card-body">
                    <div class="title" title={video.title}>{video.title}</div>
                    <div class="actions">
                        {#if video.status === 'new'}
                            <button class="btn-download" onclick={() => handleDownload(video)}>Download</button>
                            <button class="btn-ignore" onclick={() => handleIgnore(video)}>Ignore</button>
                        {:else if video.status === 'in_progress'}
                            <button class="btn-status" disabled>Queued…</button>
                        {:else if video.status === 'downloaded'}
                            <button class="btn-status downloaded" disabled>On Plex ✓</button>
                        {:else if video.status === 'ignored'}
                            <button class="btn-ignore" onclick={() => handleUnignore(video)}>Unignore</button>
                        {/if}
                    </div>
                </div>
            </div>
        {/each}
        {#if videos.length === 0 && !error}
            <p class="empty">No videos match this filter.</p>
        {/if}
    </div>
</main>

<style>
    main { padding: 1rem; font-family: sans-serif; }
    .header { display: flex; align-items: center; gap: 0.75rem; margin-bottom: 0.75rem; }
    .back { color: #666; text-decoration: none; font-size: 0.85rem; }
    .back:hover { color: #ccc; }
    .channel-name { font-weight: 600; color: #ddd; }
    .refresh { margin-left: auto; background: none; border: 1px solid #444; color: #888; padding: 0.2rem 0.6rem; border-radius: 4px; cursor: pointer; font-size: 0.8rem; }
    .refresh:hover:not(:disabled) { border-color: #4af; color: #4af; }

    .filters { display: flex; align-items: center; gap: 0.4rem; margin-bottom: 0.75rem; flex-wrap: wrap; }
    .label { font-size: 0.8rem; color: #666; }
    .pill { background: #222; color: #888; border: 1px solid #333; border-radius: 12px; padding: 0.2rem 0.7rem; font-size: 0.8rem; cursor: pointer; }
    .pill.active { background: #4af; color: #000; border-color: #4af; font-weight: 600; }
    .toggle { margin-left: auto; display: flex; align-items: center; gap: 0.3rem; font-size: 0.8rem; color: #666; cursor: pointer; }

    .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(180px, 1fr)); gap: 0.6rem; }
    .card { background: #1e1e2e; border: 1px solid #2a2a3a; border-radius: 6px; overflow: hidden; }
    .thumb { position: relative; }
    .thumb img { width: 100%; aspect-ratio: 16/9; object-fit: cover; display: block; background: #2a2a4a; }
    .badge { position: absolute; top: 4px; right: 4px; font-size: 0.6rem; font-weight: 700; padding: 2px 5px; border-radius: 3px; color: #000; }
    .card-body { padding: 0.4rem 0.5rem; }
    .title { font-size: 0.75rem; color: #ddd; line-height: 1.3; margin-bottom: 0.4rem; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
    .actions { display: flex; gap: 0.3rem; }
    .btn-download { flex: 1; background: #4af; color: #000; border: none; border-radius: 3px; padding: 0.2rem; font-size: 0.7rem; font-weight: 600; cursor: pointer; }
    .btn-ignore { background: #333; color: #777; border: none; border-radius: 3px; padding: 0.2rem 0.5rem; font-size: 0.7rem; cursor: pointer; }
    .btn-status { flex: 1; background: #222; color: #666; border: 1px solid #444; border-radius: 3px; padding: 0.2rem; font-size: 0.7rem; cursor: default; }
    .btn-status.downloaded { color: #4c4; border-color: #4c4; }
    .empty { color: #888; font-style: italic; grid-column: 1/-1; }
    .error { color: red; }
</style>
```

- [ ] **Step 2: Build**

```bash
cd /home/jamiet/code/yt-plex/web && pnpm build 2>&1 | tail -5
```

Expected: `✔ done`

- [ ] **Step 3: Commit**

```bash
cd /home/jamiet/code/yt-plex
git add web/src/routes/browse/
git commit -m "feat: /browse/[channelId] video grid with filters, download, ignore"
```

---

## Task 14: Frontend — /admin page

**Files:**
- Create: `web/src/routes/admin/+page.svelte`
- Modify: `web/src/routes/settings/+page.svelte` (replace with redirect)

- [ ] **Step 1: Create /admin page**

Create `web/src/routes/admin/+page.svelte`:

```svelte
<script lang="ts">
    import { onMount } from 'svelte';
    import {
        getSettings, updateSettings, type Settings,
        listChannels, addChannel, deleteChannel, syncChannel,
        submitJob, type Channel
    } from '$lib/api';

    // Auth guard
    onMount(async () => {
        const res = await fetch('/api/auth/me');
        if (!res.ok) window.location.href = '/login';
    });

    // Settings
    let settings = $state<Settings | null>(null);
    let settingsError = $state('');
    let settingsSaved = $state(false);
    let settingsSaving = $state(false);

    // Channels
    let channels = $state<Channel[]>([]);
    let newChannelUrl = $state('');
    let newChannelName = $state('');
    let channelError = $state('');
    let addingChannel = $state(false);

    // URL submission
    let submitUrl = $state('');
    let submitError = $state('');
    let submitSuccess = $state('');
    let submitting = $state(false);

    onMount(async () => {
        try { settings = await getSettings(); } catch { settingsError = 'Failed to load settings'; }
        try { channels = await listChannels(); } catch { /* ignore */ }
    });

    async function saveSettings() {
        if (!settings) return;
        settingsSaving = true; settingsError = ''; settingsSaved = false;
        try { await updateSettings(settings); settingsSaved = true; }
        catch (e: unknown) { settingsError = e instanceof Error ? e.message : 'Save failed'; }
        finally { settingsSaving = false; }
    }

    async function handleAddChannel() {
        if (!newChannelUrl || !newChannelName) return;
        addingChannel = true; channelError = '';
        try {
            const ch = await addChannel(newChannelUrl, newChannelName);
            channels = [...channels, ch];
            newChannelUrl = ''; newChannelName = '';
        } catch (e: unknown) {
            channelError = e instanceof Error ? e.message : 'Failed to add channel';
        } finally { addingChannel = false; }
    }

    async function handleDeleteChannel(id: string) {
        if (!confirm('Remove this channel and all its video metadata?')) return;
        try {
            await deleteChannel(id);
            channels = channels.filter(c => c.id !== id);
        } catch { /* ignore */ }
    }

    async function handleSyncChannel(id: string) {
        try { await syncChannel(id); } catch { /* ignore */ }
    }

    async function handleSubmitUrl() {
        submitError = ''; submitSuccess = '';
        submitting = true;
        try {
            await submitJob(submitUrl);
            submitSuccess = 'Queued!';
            submitUrl = '';
        } catch (e: unknown) {
            submitError = e instanceof Error ? e.message : 'Failed';
        } finally { submitting = false; }
    }
</script>

<main>
    <h2>Admin</h2>

    <!-- Channels -->
    <section>
        <h3>Approved Channels</h3>
        <div class="add-row">
            <input bind:value={newChannelName} placeholder="Display name" />
            <input bind:value={newChannelUrl} placeholder="https://youtube.com/@ChannelName" class="url-input" />
            <button onclick={handleAddChannel} disabled={addingChannel || !newChannelUrl || !newChannelName}>
                {addingChannel ? 'Adding…' : 'Add'}
            </button>
        </div>
        {#if channelError}<p class="error">{channelError}</p>{/if}
        {#if channels.length > 0}
            <table>
                <thead><tr><th>Name</th><th>URL</th><th>Last synced</th><th></th></tr></thead>
                <tbody>
                    {#each channels as ch (ch.id)}
                        <tr>
                            <td>{ch.name}</td>
                            <td class="url-cell"><a href={ch.youtube_channel_url} target="_blank" rel="noreferrer">{ch.youtube_channel_url}</a></td>
                            <td>{ch.last_synced_at ? new Date(ch.last_synced_at).toLocaleString() : 'never'}</td>
                            <td class="actions">
                                <button onclick={() => handleSyncChannel(ch.id)}>↻ Sync</button>
                                <button class="danger" onclick={() => handleDeleteChannel(ch.id)}>Remove</button>
                            </td>
                        </tr>
                    {/each}
                </tbody>
            </table>
        {:else}
            <p class="empty">No channels yet.</p>
        {/if}
    </section>

    <!-- URL submission -->
    <section>
        <h3>Submit URL</h3>
        <div class="add-row">
            <input type="url" bind:value={submitUrl} placeholder="https://www.youtube.com/watch?v=…" class="url-input" />
            <button onclick={handleSubmitUrl} disabled={submitting || !submitUrl}>{submitting ? 'Queuing…' : 'Queue'}</button>
        </div>
        {#if submitError}<p class="error">{submitError}</p>{/if}
        {#if submitSuccess}<p class="ok">{submitSuccess}</p>{/if}
    </section>

    <!-- Settings -->
    {#if settings}
    <section>
        <h3>Settings</h3>
        <form onsubmit={(e) => { e.preventDefault(); saveSettings(); }}>
            <fieldset>
                <legend>Plex</legend>
                <label>URL <input bind:value={settings.plex.url} /></label>
                <label>Token <input bind:value={settings.plex.token} /></label>
                <label>Library Section ID <input bind:value={settings.plex.library_section_id} /></label>
            </fieldset>
            <fieldset>
                <legend>Output</legend>
                <label>Base path <input bind:value={settings.output.base_path} /></label>
                <label>Path template <input bind:value={settings.output.path_template} /></label>
                <small>Variables: {'{channel}'}, {'{date}'}, {'{title}'}, {'{id}'}, {'{ext}'}</small>
            </fieldset>
            {#if settingsError}<p class="error">{settingsError}</p>{/if}
            {#if settingsSaved}<p class="ok">Saved.</p>{/if}
            <button type="submit" disabled={settingsSaving}>{settingsSaving ? 'Saving…' : 'Save settings'}</button>
        </form>
    </section>
    {/if}
</main>

<style>
    main { max-width: 720px; padding: 1rem; font-family: sans-serif; }
    h2 { margin-bottom: 1.5rem; color: #ddd; }
    h3 { color: #bbb; margin-bottom: 0.75rem; border-bottom: 1px solid #333; padding-bottom: 0.25rem; }
    section { margin-bottom: 2rem; }
    .add-row { display: flex; gap: 0.5rem; margin-bottom: 0.5rem; flex-wrap: wrap; }
    input { padding: 0.35rem 0.5rem; background: #1a1a2e; border: 1px solid #444; color: #ddd; border-radius: 4px; }
    .url-input { flex: 1; min-width: 200px; }
    button { padding: 0.35rem 0.75rem; background: #2a3a4a; border: 1px solid #4af; color: #4af; border-radius: 4px; cursor: pointer; font-size: 0.85rem; }
    button:disabled { opacity: 0.5; cursor: default; }
    button.danger { border-color: #f44; color: #f44; background: #2a1a1a; }
    table { width: 100%; border-collapse: collapse; font-size: 0.85rem; }
    th, td { text-align: left; padding: 0.35rem 0.5rem; border-bottom: 1px solid #333; }
    th { color: #888; }
    .url-cell { max-width: 280px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
    .url-cell a { color: #4af; text-decoration: none; }
    .actions { display: flex; gap: 0.4rem; white-space: nowrap; }
    fieldset { border: 1px solid #444; padding: 0.75rem; margin-bottom: 0.75rem; display: flex; flex-direction: column; gap: 0.5rem; }
    label { display: flex; flex-direction: column; gap: 0.2rem; font-size: 0.9rem; color: #bbb; }
    small { color: #666; font-size: 0.8rem; }
    .empty { color: #666; font-style: italic; font-size: 0.85rem; }
    .error { color: #f44; font-size: 0.85rem; }
    .ok { color: #4c4; font-size: 0.85rem; }
</style>
```

- [ ] **Step 2: Replace /settings with redirect to /admin**

Replace `web/src/routes/settings/+page.svelte`:

```svelte
<script lang="ts">
    import { goto } from '$app/navigation';
    goto('/admin', { replaceState: true });
</script>
```

- [ ] **Step 3: Build**

```bash
cd /home/jamiet/code/yt-plex/web && pnpm build 2>&1 | tail -5
```

Expected: `✔ done`

- [ ] **Step 4: Commit**

```bash
cd /home/jamiet/code/yt-plex
git add web/src/routes/admin/+page.svelte web/src/routes/settings/+page.svelte
git commit -m "feat: /admin page (channels, URL submission, settings)"
```

---

## Task 15: Final build and verification

- [ ] **Step 1: Run full Rust test suite**

```bash
cd /home/jamiet/code/yt-plex && cargo test 2>&1 | grep -E "^test result|FAILED"
```

Expected: all crates pass, 0 failures.

- [ ] **Step 2: Release build**

```bash
cd /home/jamiet/code/yt-plex && cargo build --release -p yt-plex-server 2>&1 | grep "^error" | head -5
```

Expected: clean.

- [ ] **Step 3: Frontend production build**

```bash
cd /home/jamiet/code/yt-plex/web && pnpm build 2>&1 | tail -5
```

Expected: `✔ done`

- [ ] **Step 4: Final commit**

```bash
cd /home/jamiet/code/yt-plex
git add docs/superpowers/plans/2026-04-05-channel-browser.md
git commit -m "docs: add channel browser implementation plan"
```
