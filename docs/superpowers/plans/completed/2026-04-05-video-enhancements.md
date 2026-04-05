# Video Enhancements Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add four connected improvements to the video browse experience: FTS title/description search, infinite scroll pagination, a video detail page with description, and multi-select bulk download/ignore.

**Architecture:** Add a `description` column and FTS5 virtual table (with triggers) to the SQLite schema; lazy-fetch descriptions on first detail-page visit via yt-dlp; extend `list_videos_for_channel` with `search`, `limit`, and `offset` params returning a `VideoPage`; add `GET /api/videos/{youtube_id}`; rework the frontend video grid to use IntersectionObserver paging, a search input, and checkboxes with a bulk-action bar; add a detail route.

**Tech Stack:** Rust, rusqlite (SQLite FTS5), tokio, yt-dlp (CLI for description fetch), SvelteKit 5 (Svelte runes), TypeScript, browser IntersectionObserver API.

---

## File Map

| File | Change |
|---|---|
| `crates/common/src/models.rs` | Add `description: Option<String>` to `Video`; add `VideoPage` struct |
| `crates/server/src/db.rs` | Add FTS5 table + triggers to SCHEMA; add `get_video`, `set_video_description`; update `list_videos_for_channel` signature (search, limit, offset → `VideoPage`) |
| `crates/server/src/sync.rs` | Add `fetch_video_description(youtube_id) -> Result<String>` |
| `crates/server/src/routes/channels.rs` | Accept `q`, `limit`, `offset` in `list_channel_videos` |
| `crates/server/src/routes/videos.rs` | Add `get_video` handler (lazy description) |
| `crates/server/src/lib.rs` | Add route `GET /api/videos/{youtube_id}` |
| `web/src/lib/api.ts` | Add `description` to `Video`; add `VideoPage`; add `getVideo`; update `listVideos` signature |
| `web/src/routes/browse/[channelId]/+page.svelte` | Add search input; replace individual buttons with checkboxes + bulk action bar; infinite scroll via IntersectionObserver |
| `web/src/routes/browse/[channelId]/[videoId]/+page.svelte` | **Create** — detail page (thumbnail, title, description, status, Download/Ignore) |

---

## Task 1: Add `description` to `Video` model and `VideoPage` struct

**Files:**
- Modify: `crates/common/src/models.rs`

- [ ] **Step 1: Write failing tests**

Add to `#[cfg(test)] mod tests` in `crates/common/src/models.rs`:

```rust
#[test]
fn video_has_description_field() {
    let v = Video {
        youtube_id: "abc".into(),
        channel_id: "ch1".into(),
        title: "Title".into(),
        published_at: None,
        downloaded_at: None,
        last_seen_at: "2026-04-05T00:00:00Z".into(),
        ignored_at: None,
        status: VideoStatus::New,
        description: Some("A description".into()),
    };
    let json = serde_json::to_string(&v).unwrap();
    assert!(json.contains("\"description\":\"A description\""));
}

#[test]
fn video_page_serialises() {
    let page = VideoPage {
        videos: vec![],
        has_more: false,
    };
    let json = serde_json::to_string(&page).unwrap();
    assert!(json.contains("\"has_more\":false"));
}
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cd /home/jamiet/code/yt-plex && cargo test -p yt-plex-common video_has_description 2>&1 | tail -5
```

Expected: compile error — `Video` struct has no `description` field, `VideoPage` not defined.

- [ ] **Step 3: Add `description` to `Video` and add `VideoPage`**

In `crates/common/src/models.rs`, update the `Video` struct:

```rust
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
    pub description: Option<String>,
}
```

Add after the `Video` struct:

```rust
/// Paginated response for list_videos_for_channel.
#[derive(Debug, Serialize, Deserialize)]
pub struct VideoPage {
    pub videos: Vec<Video>,
    pub has_more: bool,
}
```

- [ ] **Step 4: Fix existing compile errors in server crate**

The `Video` struct now has a new required field. Run:

```bash
cd /home/jamiet/code/yt-plex && cargo build 2>&1 | grep "^error" | head -20
```

All `Video { ... }` struct literals in `crates/server/src/db.rs` (the row mapper in `list_videos_for_channel`) must add `description: row.get(8)?`. Update that mapper — do **not** add column 8 to the SELECT yet; that happens in Task 3. For now just add `description: None` to silence the compiler:

In `crates/server/src/db.rs`, find the row mapper in `list_videos_for_channel`:

```rust
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
```

Replace with:

```rust
            Ok(Video {
                youtube_id: row.get(0)?,
                channel_id: row.get(1)?,
                title: row.get(2)?,
                published_at: row.get(3)?,
                downloaded_at: row.get(4)?,
                last_seen_at: row.get(5)?,
                ignored_at: row.get(6)?,
                status,
                description: None,  // populated lazily on detail page visit
            })
```

- [ ] **Step 5: Run all tests**

```bash
cd /home/jamiet/code/yt-plex && cargo test 2>&1 | grep -E "^test result|FAILED"
```

Expected: all pass.

- [ ] **Step 6: Commit**

```bash
cd /home/jamiet/code/yt-plex
git add crates/common/src/models.rs crates/server/src/db.rs
git commit -m "feat: add description field to Video model and VideoPage struct"
```

---

## Task 2: DB schema — FTS5 table and triggers

**Files:**
- Modify: `crates/server/src/db.rs` (SCHEMA constant only)

- [ ] **Step 1: Write failing test**

Add to `#[cfg(test)] mod tests` in `crates/server/src/db.rs`:

```rust
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
```

- [ ] **Step 2: Run to confirm they fail**

```bash
cd /home/jamiet/code/yt-plex && cargo test -p yt-plex-server fts_table_exists 2>&1 | tail -5
```

Expected: test fails — table `videos_fts` does not exist.

- [ ] **Step 3: Add FTS5 table and triggers to SCHEMA**

In `crates/server/src/db.rs`, append to the `SCHEMA` constant before the closing `"`:

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
    ignored_at    TEXT,
    description   TEXT
);

CREATE INDEX IF NOT EXISTS idx_videos_channel_id ON videos(channel_id);
CREATE INDEX IF NOT EXISTS idx_videos_published_at ON videos(published_at DESC);

-- FTS5 virtual table for title+description search (content-table backed, triggers keep in sync)
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
";
```

- [ ] **Step 4: Also add `description TEXT` column to the existing `videos` table definition**

Already done in Step 3 above (the `CREATE TABLE IF NOT EXISTS videos` block now includes `description TEXT`).

For the existing live database (dev machine), the column won't be added automatically by `CREATE TABLE IF NOT EXISTS` since the table already exists. This is fine — we're in initial development so wiping the DB is acceptable. Delete `~/.local/share/yt-plex/db.sqlite` before restarting the server.

- [ ] **Step 5: Run all server tests**

```bash
cd /home/jamiet/code/yt-plex && cargo test -p yt-plex-server 2>&1 | grep -E "^test result|FAILED"
```

Expected: all pass including `fts_table_exists` and `fts_indexes_on_insert`.

- [ ] **Step 6: Commit**

```bash
cd /home/jamiet/code/yt-plex
git add crates/server/src/db.rs
git commit -m "feat: add description column and FTS5 index to videos schema"
```

---

## Task 3: DB operations — `get_video`, `set_video_description`, and updated `list_videos_for_channel`

**Files:**
- Modify: `crates/server/src/db.rs`

- [ ] **Step 1: Write failing tests**

Add to `#[cfg(test)] mod tests` in `crates/server/src/db.rs`:

```rust
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
    let page = db.list_videos_for_channel(&ch.id, VideoFilter::All, false, Some("Rust"), 50, 0).unwrap();
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
    let page1 = db.list_videos_for_channel(&ch.id, VideoFilter::All, false, None, 3, 0).unwrap();
    assert_eq!(page1.videos.len(), 3);
    assert!(page1.has_more);

    let page2 = db.list_videos_for_channel(&ch.id, VideoFilter::All, false, None, 3, 3).unwrap();
    assert_eq!(page2.videos.len(), 2);
    assert!(!page2.has_more);
}
```

- [ ] **Step 2: Run to confirm they fail**

```bash
cd /home/jamiet/code/yt-plex && cargo test -p yt-plex-server get_video_returns_none 2>&1 | tail -5
```

Expected: compile error — methods not defined, wrong `list_videos_for_channel` arity.

- [ ] **Step 3: Implement `get_video` and `set_video_description`**

Add to the `impl Db` block in `crates/server/src/db.rs`:

```rust
pub fn get_video(&self, youtube_id: &str) -> Result<Option<Video>> {
    let active_job_subq = "EXISTS(SELECT 1 FROM jobs WHERE url = 'https://www.youtube.com/watch?v=' || v.youtube_id AND status IN ('queued','downloading','copying'))";
    let sql = format!(
        "SELECT v.youtube_id, v.channel_id, v.title, v.published_at,
                v.downloaded_at, v.last_seen_at, v.ignored_at,
                CASE
                    WHEN {active_job_subq} THEN 'in_progress'
                    WHEN v.downloaded_at IS NOT NULL THEN 'downloaded'
                    WHEN v.ignored_at IS NOT NULL THEN 'ignored'
                    ELSE 'new'
                END as derived_status,
                v.description
         FROM videos v
         WHERE v.youtube_id = ?1"
    );
    let conn = self.conn.lock().unwrap();
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query_map(rusqlite::params![youtube_id], |row| {
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
```

- [ ] **Step 4: Update `list_videos_for_channel` signature and return type**

Replace the existing `list_videos_for_channel` method with:

```rust
pub fn list_videos_for_channel(
    &self,
    channel_id: &str,
    filter: VideoFilter,
    show_ignored: bool,
    search: Option<&str>,
    limit: usize,
    offset: usize,
) -> Result<VideoPage> {
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

    // ?2 IS NULL short-circuits before MATCH is evaluated, so NULL search is safe.
    let sql = format!(
        "SELECT v.youtube_id, v.channel_id, v.title, v.published_at,
                v.downloaded_at, v.last_seen_at, v.ignored_at,
                CASE
                    WHEN {active_job_subq} THEN 'in_progress'
                    WHEN v.downloaded_at IS NOT NULL THEN 'downloaded'
                    WHEN v.ignored_at IS NOT NULL THEN 'ignored'
                    ELSE 'new'
                END as derived_status,
                v.description
         FROM videos v
         WHERE v.channel_id = ?1
           AND (?2 IS NULL OR v.rowid IN (SELECT rowid FROM videos_fts WHERE videos_fts MATCH ?2))
           AND ({filter_cond})
         ORDER BY v.published_at DESC NULLS LAST, v.last_seen_at DESC
         LIMIT ?3 OFFSET ?4"
    );

    let conn = self.conn.lock().unwrap();
    let mut stmt = conn.prepare(&sql)?;
    // Fetch limit+1 to detect whether there are more pages.
    let fetch_limit = (limit + 1) as i64;
    let rows = stmt.query_map(
        rusqlite::params![channel_id, search, fetch_limit, offset as i64],
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
```

Note: the `use yt_plex_common::models::{..., VideoPage};` import must be added at the top of `db.rs`.

- [ ] **Step 5: Fix broken call site in `routes/channels.rs`**

`list_channel_videos` in `routes/channels.rs` calls `list_videos_for_channel` with the old 3-arg signature. Update it to pass dummy values for now:

```rust
match state.db.list_videos_for_channel(&id, filter, show_ignored, None, 50, 0) {
```

This will be properly wired in Task 5.

- [ ] **Step 6: Fix broken tests**

The existing tests that call `list_videos_for_channel` with the old 3-arg signature need updating. Search for all calls in `db.rs` tests and add `None, 50, 0` (or appropriate values) as the last three args. Also update tests that construct `Video` structs to add `description: None`. Also update the `get_video` return — it now returns `VideoPage`, so update tests accordingly:

For example, change:
```rust
let videos = db.list_videos_for_channel(&ch.id, VideoFilter::All, false).unwrap();
```
to:
```rust
let page = db.list_videos_for_channel(&ch.id, VideoFilter::All, false, None, 50, 0).unwrap();
let videos = page.videos;
```

Apply this change to all affected tests: `upsert_and_list_videos_new`, `set_video_downloaded_changes_status`, `ignore_hides_from_new_filter`, `unignore_makes_video_new_again`, `videos_ordered_by_published_at_desc`, `all_filter_excludes_ignored_by_default`.

- [ ] **Step 7: Run all tests**

```bash
cd /home/jamiet/code/yt-plex && cargo test 2>&1 | grep -E "^test result|FAILED"
```

Expected: all pass.

- [ ] **Step 8: Commit**

```bash
cd /home/jamiet/code/yt-plex
git add crates/common/src/models.rs crates/server/src/db.rs crates/server/src/routes/channels.rs
git commit -m "feat: add get_video, set_video_description; paginated+searchable list_videos_for_channel"
```

---

## Task 4: `fetch_video_description` in sync module

**Files:**
- Modify: `crates/server/src/sync.rs`

- [ ] **Step 1: Write failing test**

Add to `#[cfg(test)] mod tests` in `crates/server/src/sync.rs`:

```rust
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
```

- [ ] **Step 2: Run to confirm they fail**

```bash
cd /home/jamiet/code/yt-plex && cargo test -p yt-plex-server parse_description_from_json 2>&1 | tail -5
```

Expected: compile error — `YtDlpVideoMeta` not defined.

- [ ] **Step 3: Implement**

Add to `crates/server/src/sync.rs` (after the existing `use` statements):

```rust
use serde::Deserialize;
```

Add the struct and function:

```rust
#[derive(Deserialize)]
struct YtDlpVideoMeta {
    description: Option<String>,
}

/// Fetch the description for a single video by calling yt-dlp -j.
/// Runs synchronously (blocking tokio task). Takes ~2–5 seconds.
pub async fn fetch_video_description(youtube_id: &str) -> Result<String> {
    let url = format!("https://www.youtube.com/watch?v={youtube_id}");
    let output = Command::new("yt-dlp")
        .args(["--no-playlist", "-j", &url])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .await
        .context("spawning yt-dlp for description fetch")?;

    if !output.status.success() {
        anyhow::bail!("yt-dlp exited with status {}", output.status);
    }
    let meta: YtDlpVideoMeta = serde_json::from_slice(&output.stdout)
        .context("parsing yt-dlp JSON")?;
    Ok(meta.description.unwrap_or_default())
}
```

Note: `Command::output()` is used (not `spawn` + stream) because `-j` outputs one JSON blob. The stdout buffer won't overflow for a single video's metadata.

- [ ] **Step 4: Run all tests**

```bash
cd /home/jamiet/code/yt-plex && cargo test -p yt-plex-server 2>&1 | grep -E "^test result|FAILED"
```

Expected: all pass.

- [ ] **Step 5: Commit**

```bash
cd /home/jamiet/code/yt-plex
git add crates/server/src/sync.rs
git commit -m "feat: add fetch_video_description via yt-dlp -j"
```

---

## Task 5: Backend routes — video detail endpoint + updated list_channel_videos

**Files:**
- Modify: `crates/server/src/routes/videos.rs`
- Modify: `crates/server/src/routes/channels.rs`
- Modify: `crates/server/src/lib.rs`

- [ ] **Step 1: Add `GET /api/videos/{youtube_id}` handler**

Replace the full contents of `crates/server/src/routes/videos.rs`:

```rust
use axum::{extract::{Path, State}, http::StatusCode, response::IntoResponse, Json};
use chrono::Utc;
use tracing::error;
use crate::{sync, AppState};

pub async fn get_video(
    State(state): State<AppState>,
    Path(youtube_id): Path<String>,
) -> impl IntoResponse {
    // Check video exists in DB
    let video = match state.db.get_video(&youtube_id) {
        Ok(Some(v)) => v,
        Ok(None) => return (StatusCode::NOT_FOUND, "Video not found").into_response(),
        Err(e) => {
            error!("get_video: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Server error").into_response();
        }
    };
    // If description not yet cached, fetch it now (synchronous, ~3s)
    if video.description.is_none() {
        match sync::fetch_video_description(&youtube_id).await {
            Ok(desc) => {
                if let Err(e) = state.db.set_video_description(&youtube_id, &desc) {
                    error!("set_video_description: {e}");
                }
                // Re-fetch to get updated description
                return match state.db.get_video(&youtube_id) {
                    Ok(Some(v)) => Json(v).into_response(),
                    Ok(None) => (StatusCode::NOT_FOUND, "Video not found").into_response(),
                    Err(e) => {
                        error!("get_video after desc update: {e}");
                        (StatusCode::INTERNAL_SERVER_ERROR, "Server error").into_response()
                    }
                };
            }
            Err(e) => {
                // Non-fatal: return the video without description
                error!("fetch_video_description for {youtube_id}: {e:#}");
            }
        }
    }
    Json(video).into_response()
}

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

- [ ] **Step 2: Wire the new route in lib.rs**

In `crates/server/src/lib.rs`, add to `build_router`:

```rust
        .route("/api/videos/{youtube_id}", get(routes::videos::get_video))
```

Place it before the ignore routes:

```rust
        .route("/api/videos/{youtube_id}", get(routes::videos::get_video))
        .route("/api/videos/{youtube_id}/ignore", post(routes::videos::ignore_video))
        .route("/api/videos/{youtube_id}/ignore", delete(routes::videos::unignore_video))
```

Add `get` to the existing routing import if not already present:
```rust
use axum::routing::{delete, get, post, put};
```

- [ ] **Step 3: Update `list_channel_videos` route to accept `q`, `limit`, `offset`**

In `crates/server/src/routes/channels.rs`, replace `VideoQueryParams` and `list_channel_videos`:

```rust
pub async fn list_channel_videos(
    State(state): State<AppState>,
    Path(id): Path<String>,
    axum::extract::Query(params): axum::extract::Query<VideoQueryParams>,
) -> impl IntoResponse {
    let filter = yt_plex_common::models::VideoFilter::from_str(
        params.filter.as_deref().unwrap_or("new"),
    );
    let show_ignored = params.show_ignored.unwrap_or(false);
    let search = params.q.as_deref().filter(|s| !s.is_empty());
    let limit = params.limit.unwrap_or(48).min(200);
    let offset = params.offset.unwrap_or(0);
    match state.db.list_videos_for_channel(&id, filter, show_ignored, search, limit, offset) {
        Ok(page) => Json(page).into_response(),
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
    pub q: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}
```

- [ ] **Step 4: Build**

```bash
cd /home/jamiet/code/yt-plex && cargo build -p yt-plex-server 2>&1 | grep "^error" | head -20
```

Expected: clean.

- [ ] **Step 5: Run all tests**

```bash
cd /home/jamiet/code/yt-plex && cargo test 2>&1 | grep -E "^test result|FAILED"
```

Expected: all pass.

- [ ] **Step 6: Commit**

```bash
cd /home/jamiet/code/yt-plex
git add crates/server/src/routes/videos.rs crates/server/src/routes/channels.rs crates/server/src/lib.rs
git commit -m "feat: add GET /api/videos/{id} with lazy description; accept q/limit/offset in video list"
```

---

## Task 6: Frontend API types

**Files:**
- Modify: `web/src/lib/api.ts`

- [ ] **Step 1: Update `api.ts`**

Update the `Video` interface to add `description`, add `VideoPage`, update `listVideos`, and add `getVideo`. Replace the relevant blocks:

Add `description` to `Video`:

```typescript
export interface Video {
    youtube_id: string;
    channel_id: string;
    title: string;
    published_at: string | null;
    downloaded_at: string | null;
    last_seen_at: string;
    ignored_at: string | null;
    status: VideoStatus;
    description: string | null;
}
```

Add `VideoPage` after `Video`:

```typescript
export interface VideoPage {
    videos: Video[];
    has_more: boolean;
}
```

Replace `listVideos`:

```typescript
export async function listVideos(
    channelId: string,
    filter: 'new' | 'downloaded' | 'all' = 'new',
    showIgnored = false,
    search = '',
    limit = 48,
    offset = 0,
): Promise<VideoPage> {
    const params = new URLSearchParams({ filter, limit: String(limit), offset: String(offset) });
    if (showIgnored) params.set('show_ignored', 'true');
    if (search) params.set('q', search);
    const res = await fetch(`/api/channels/${channelId}/videos?${params}`);
    if (!res.ok) throw new Error(`listVideos failed: ${res.status}`);
    return res.json();
}
```

Add `getVideo` after `unignoreVideo`:

```typescript
export async function getVideo(youtubeId: string): Promise<Video> {
    const res = await fetch(`/api/videos/${youtubeId}`);
    if (!res.ok) throw new Error(`getVideo failed: ${res.status}`);
    return res.json();
}
```

- [ ] **Step 2: Build frontend**

```bash
cd /home/jamiet/code/yt-plex/web && pnpm build 2>&1 | tail -5
```

Fix any TypeScript errors before continuing.

- [ ] **Step 3: Commit**

```bash
cd /home/jamiet/code/yt-plex
git add web/src/lib/api.ts
git commit -m "feat: update Video type with description; add VideoPage and getVideo"
```

---

## Task 7: Video grid — search input, infinite scroll, multi-select bulk actions

**Files:**
- Modify: `web/src/routes/browse/[channelId]/+page.svelte`

This replaces the entire page with a significantly reworked version. The key changes are:

1. **Search**: debounced text input; changing it resets to page 0 and reloads
2. **Infinite scroll**: `IntersectionObserver` watches a sentinel `<div>` below the grid; when visible and `has_more`, appends the next page
3. **Multi-select + bulk actions**: each card has a checkbox; a sticky bar shows when anything is selected offering Download All and Ignore All

- [ ] **Step 1: Replace the page**

Write `web/src/routes/browse/[channelId]/+page.svelte` with:

```svelte
<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { page } from '$app/stores';
    import {
        listChannels, listVideos, ignoreVideo, unignoreVideo,
        submitJobByYoutubeId, syncChannel,
        type Channel, type Video, type VideoStatus
    } from '$lib/api';
    import { createWsStore } from '$lib/ws';

    const channelId = $derived($page.params.channelId);

    let channel = $state<Channel | null>(null);
    let videos = $state<Video[]>([]);
    let filter = $state<'new' | 'downloaded' | 'all'>('new');
    let showIgnored = $state(false);
    let search = $state('');
    let error = $state('');
    let syncing = $state(false);
    let loading = $state(false);
    let hasMore = $state(false);
    let offset = $state(0);
    const LIMIT = 48;

    // Multi-select
    let selected = $state(new Set<string>());
    let bulkWorking = $state(false);

    const ws = createWsStore();
    let unsubWs: (() => void) | undefined;

    // Debounce search: only reload after 300ms idle
    let searchTimer: ReturnType<typeof setTimeout> | null = null;

    // Sentinel element for IntersectionObserver
    let sentinel: HTMLDivElement | undefined;
    let observer: IntersectionObserver | null = null;

    async function loadPage(reset: boolean) {
        if (loading) return;
        loading = true;
        const currentOffset = reset ? 0 : offset;
        try {
            const result = await listVideos(channelId, filter, showIgnored, search, LIMIT, currentOffset);
            if (reset) {
                videos = result.videos;
                selected = new Set();
            } else {
                videos = [...videos, ...result.videos];
            }
            hasMore = result.has_more;
            offset = currentOffset + result.videos.length;
        } catch (e: unknown) {
            error = e instanceof Error ? e.message : 'Failed to load videos';
        } finally {
            loading = false;
        }
    }

    function resetAndLoad() {
        offset = 0;
        hasMore = false;
        loadPage(true);
    }

    onMount(async () => {
        try {
            const channels = await listChannels();
            channel = channels.find(c => c.id === channelId) ?? null;
        } catch { /* ignore */ }
        await loadPage(true);
        ws.connect();
        unsubWs = ws.subscribe(() => {});

        // Set up IntersectionObserver on the sentinel
        observer = new IntersectionObserver((entries) => {
            if (entries[0].isIntersecting && hasMore && !loading) {
                loadPage(false);
            }
        }, { rootMargin: '200px' });
        if (sentinel) observer.observe(sentinel);
    });

    onDestroy(() => {
        ws.disconnect();
        unsubWs?.();
        observer?.disconnect();
        if (searchTimer) clearTimeout(searchTimer);
    });

    // Re-observe sentinel when it mounts (after first render)
    $effect(() => {
        if (sentinel && observer) observer.observe(sentinel);
    });

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

    // Reload when filter or showIgnored changes
    $effect(() => {
        void filter;
        void showIgnored;
        resetAndLoad();
    });

    function handleSearchInput(e: Event) {
        search = (e.target as HTMLInputElement).value;
        if (searchTimer) clearTimeout(searchTimer);
        searchTimer = setTimeout(resetAndLoad, 300);
    }

    async function handleDownload(youtubeId: string) {
        try {
            await submitJobByYoutubeId(youtubeId);
            videos = videos.map(v =>
                v.youtube_id === youtubeId ? { ...v, status: 'in_progress' as VideoStatus } : v
            );
        } catch (e: unknown) {
            alert(e instanceof Error ? e.message : 'Failed to queue download');
        }
    }

    async function handleIgnore(youtubeId: string) {
        try {
            await ignoreVideo(youtubeId);
            videos = videos.map(v =>
                v.youtube_id === youtubeId ? { ...v, status: 'ignored' as VideoStatus, ignored_at: new Date().toISOString() } : v
            );
            selected = new Set([...selected].filter(id => id !== youtubeId));
        } catch { /* ignore */ }
    }

    async function handleUnignore(youtubeId: string) {
        try {
            await unignoreVideo(youtubeId);
            videos = videos.map(v =>
                v.youtube_id === youtubeId ? { ...v, status: 'new' as VideoStatus, ignored_at: null } : v
            );
        } catch { /* ignore */ }
    }

    async function handleSync() {
        syncing = true;
        try {
            await syncChannel(channelId);
            setTimeout(resetAndLoad, 2000);
        } catch { /* ignore */ } finally {
            syncing = false;
        }
    }

    function toggleSelect(youtubeId: string) {
        const next = new Set(selected);
        if (next.has(youtubeId)) next.delete(youtubeId);
        else next.add(youtubeId);
        selected = next;
    }

    async function bulkDownload() {
        bulkWorking = true;
        for (const id of selected) {
            const v = videos.find(v => v.youtube_id === id);
            if (v?.status === 'new') await handleDownload(id).catch(() => {});
        }
        selected = new Set();
        bulkWorking = false;
    }

    async function bulkIgnore() {
        bulkWorking = true;
        for (const id of selected) {
            await handleIgnore(id).catch(() => {});
        }
        selected = new Set();
        bulkWorking = false;
    }

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

    <div class="toolbar">
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
        <input
            class="search"
            type="search"
            placeholder="Search titles…"
            value={search}
            oninput={handleSearchInput}
        />
    </div>

    {#if selected.size > 0}
        <div class="bulk-bar">
            <span>{selected.size} selected</span>
            <button onclick={bulkDownload} disabled={bulkWorking}>↓ Download all</button>
            <button onclick={bulkIgnore} disabled={bulkWorking}>✕ Ignore all</button>
            <button class="clear" onclick={() => selected = new Set()}>Clear</button>
        </div>
    {/if}

    {#if error}<p class="error">{error}</p>{/if}

    <div class="grid">
        {#each videos as video (video.youtube_id)}
            {@const isSelected = selected.has(video.youtube_id)}
            <div class="card" class:card-selected={isSelected}>
                <label class="check-wrap" title="Select">
                    <input
                        type="checkbox"
                        class="card-check"
                        checked={isSelected}
                        onchange={() => toggleSelect(video.youtube_id)}
                    />
                </label>
                <a href="/browse/{channelId}/{video.youtube_id}" class="thumb-link">
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
                </a>
                <div class="card-body">
                    <a href="/browse/{channelId}/{video.youtube_id}" class="title" title={video.title}>
                        {video.title}
                    </a>
                    <div class="actions">
                        {#if video.status === 'new'}
                            <button class="btn-download" onclick={() => handleDownload(video.youtube_id)}>Download</button>
                            <button class="btn-ignore" onclick={() => handleIgnore(video.youtube_id)}>✕</button>
                        {:else if video.status === 'in_progress'}
                            <button class="btn-status" disabled>Queued…</button>
                        {:else if video.status === 'downloaded'}
                            <button class="btn-status downloaded" disabled>On Plex ✓</button>
                        {:else if video.status === 'ignored'}
                            <button class="btn-ignore" onclick={() => handleUnignore(video.youtube_id)}>Unignore</button>
                        {/if}
                    </div>
                </div>
            </div>
        {/each}
        {#if videos.length === 0 && !loading && !error}
            <p class="empty">No videos match this filter.</p>
        {/if}
    </div>

    {#if loading}
        <p class="loading-msg">Loading…</p>
    {/if}

    <!-- Sentinel: observed by IntersectionObserver to trigger next page -->
    <div bind:this={sentinel} class="sentinel"></div>
</main>

<style>
    main { padding: 1rem; font-family: sans-serif; }
    .header { display: flex; align-items: center; gap: 0.75rem; margin-bottom: 0.75rem; }
    .back { color: #666; text-decoration: none; font-size: 0.85rem; }
    .back:hover { color: #ccc; }
    .channel-name { font-weight: 600; color: #ddd; }
    .refresh { margin-left: auto; background: none; border: 1px solid #444; color: #888; padding: 0.2rem 0.6rem; border-radius: 4px; cursor: pointer; font-size: 0.8rem; }
    .refresh:hover:not(:disabled) { border-color: #4af; color: #4af; }

    .toolbar { display: flex; align-items: center; gap: 0.75rem; margin-bottom: 0.75rem; flex-wrap: wrap; }
    .filters { display: flex; align-items: center; gap: 0.4rem; flex-wrap: wrap; }
    .label { font-size: 0.8rem; color: #666; }
    .pill { background: #222; color: #888; border: 1px solid #333; border-radius: 12px; padding: 0.2rem 0.7rem; font-size: 0.8rem; cursor: pointer; }
    .pill.active { background: #4af; color: #000; border-color: #4af; font-weight: 600; }
    .toggle { display: flex; align-items: center; gap: 0.3rem; font-size: 0.8rem; color: #666; cursor: pointer; }
    .search { margin-left: auto; padding: 0.3rem 0.6rem; background: #1a1a2e; border: 1px solid #444; color: #ddd; border-radius: 16px; font-size: 0.85rem; min-width: 180px; }
    .search:focus { outline: none; border-color: #4af; }

    .bulk-bar {
        display: flex; align-items: center; gap: 0.5rem;
        background: #1e2a3a; border: 1px solid #4af; border-radius: 6px;
        padding: 0.4rem 0.75rem; margin-bottom: 0.75rem; font-size: 0.85rem; color: #ddd;
    }
    .bulk-bar span { margin-right: auto; }
    .bulk-bar button { background: #2a4a6a; border: 1px solid #4af; color: #4af; border-radius: 4px; padding: 0.2rem 0.6rem; cursor: pointer; font-size: 0.8rem; }
    .bulk-bar button:disabled { opacity: 0.5; cursor: default; }
    .bulk-bar button.clear { border-color: #888; color: #888; background: none; }

    .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(180px, 1fr)); gap: 0.6rem; }
    .card { background: #1e1e2e; border: 1px solid #2a2a3a; border-radius: 6px; overflow: hidden; position: relative; }
    .card-selected { border-color: #4af; box-shadow: 0 0 0 1px #4af; }
    .check-wrap { position: absolute; top: 4px; left: 4px; z-index: 2; cursor: pointer; }
    .card-check { width: 16px; height: 16px; accent-color: #4af; cursor: pointer; }
    .thumb-link { display: block; text-decoration: none; }
    .thumb { position: relative; }
    .thumb img { width: 100%; aspect-ratio: 16/9; object-fit: cover; display: block; background: #2a2a4a; }
    .badge { position: absolute; top: 4px; right: 4px; font-size: 0.6rem; font-weight: 700; padding: 2px 5px; border-radius: 3px; color: #000; }
    .card-body { padding: 0.4rem 0.5rem; }
    .title { font-size: 0.75rem; color: #ddd; line-height: 1.3; margin-bottom: 0.4rem; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; text-decoration: none; }
    .title:hover { color: #4af; }
    .actions { display: flex; gap: 0.3rem; }
    .btn-download { flex: 1; background: #4af; color: #000; border: none; border-radius: 3px; padding: 0.2rem; font-size: 0.7rem; font-weight: 600; cursor: pointer; }
    .btn-ignore { background: #333; color: #777; border: none; border-radius: 3px; padding: 0.2rem 0.5rem; font-size: 0.7rem; cursor: pointer; }
    .btn-status { flex: 1; background: #222; color: #666; border: 1px solid #444; border-radius: 3px; padding: 0.2rem; font-size: 0.7rem; cursor: default; }
    .btn-status.downloaded { color: #4c4; border-color: #4c4; }
    .empty, .loading-msg { color: #888; font-style: italic; grid-column: 1/-1; text-align: center; padding: 2rem; }
    .error { color: red; }
    .sentinel { height: 1px; }
</style>
```

- [ ] **Step 2: Build**

```bash
cd /home/jamiet/code/yt-plex/web && pnpm build 2>&1 | tail -5
```

Fix any TypeScript errors.

- [ ] **Step 3: Commit**

```bash
cd /home/jamiet/code/yt-plex
git add web/src/routes/browse/[channelId]/+page.svelte
git commit -m "feat: video grid — search input, infinite scroll, multi-select bulk actions"
```

---

## Task 8: Video detail page

**Files:**
- Create: `web/src/routes/browse/[channelId]/[videoId]/+page.svelte`

- [ ] **Step 1: Create the directory and file**

Create `web/src/routes/browse/[channelId]/[videoId]/+page.svelte`:

```svelte
<script lang="ts">
    import { onMount } from 'svelte';
    import { page } from '$app/stores';
    import {
        getVideo, submitJobByYoutubeId, ignoreVideo, unignoreVideo,
        type Video, type VideoStatus
    } from '$lib/api';

    const channelId = $derived($page.params.channelId);
    const videoId = $derived($page.params.videoId);

    let video = $state<Video | null>(null);
    let loading = $state(true);
    let error = $state('');
    let actionWorking = $state(false);
    let actionMsg = $state('');

    onMount(async () => {
        try {
            video = await getVideo(videoId);
        } catch (e: unknown) {
            error = e instanceof Error ? e.message : 'Failed to load video';
        } finally {
            loading = false;
        }
    });

    async function handleDownload() {
        if (!video) return;
        actionWorking = true; actionMsg = '';
        try {
            await submitJobByYoutubeId(video.youtube_id);
            video = { ...video, status: 'in_progress' as VideoStatus };
            actionMsg = 'Queued for download!';
        } catch (e: unknown) {
            actionMsg = e instanceof Error ? e.message : 'Failed';
        } finally { actionWorking = false; }
    }

    async function handleIgnore() {
        if (!video) return;
        actionWorking = true;
        try {
            await ignoreVideo(video.youtube_id);
            video = { ...video, status: 'ignored' as VideoStatus, ignored_at: new Date().toISOString() };
        } catch { /* ignore */ } finally { actionWorking = false; }
    }

    async function handleUnignore() {
        if (!video) return;
        actionWorking = true;
        try {
            await unignoreVideo(video.youtube_id);
            video = { ...video, status: 'new' as VideoStatus, ignored_at: null };
        } catch { /* ignore */ } finally { actionWorking = false; }
    }

    const statusLabel: Record<VideoStatus, string> = {
        new: 'Not downloaded',
        in_progress: 'Downloading…',
        downloaded: 'On Plex ✓',
        ignored: 'Ignored',
    };
    const statusColour: Record<VideoStatus, string> = {
        new: '#4af',
        in_progress: '#fa4',
        downloaded: '#4c4',
        ignored: '#555',
    };
</script>

<main>
    <a href="/browse/{channelId}" class="back">← Back to channel</a>

    {#if loading}
        <p class="loading">Loading…</p>
    {:else if error}
        <p class="error">{error}</p>
    {:else if video}
        <div class="detail">
            <img
                class="thumb"
                src="https://img.youtube.com/vi/{video.youtube_id}/hqdefault.jpg"
                alt={video.title}
            />
            <div class="info">
                <h1 class="title">{video.title}</h1>
                {#if video.published_at}
                    <p class="meta">Published {new Date(video.published_at).toLocaleDateString()}</p>
                {/if}
                <p class="status" style="color:{statusColour[video.status]}">
                    {statusLabel[video.status]}
                </p>
                <div class="actions">
                    {#if video.status === 'new'}
                        <button class="btn-primary" onclick={handleDownload} disabled={actionWorking}>
                            ↓ Download
                        </button>
                        <button class="btn-secondary" onclick={handleIgnore} disabled={actionWorking}>
                            Ignore
                        </button>
                    {:else if video.status === 'in_progress'}
                        <button class="btn-disabled" disabled>Queued…</button>
                    {:else if video.status === 'downloaded'}
                        <button class="btn-disabled downloaded" disabled>On Plex ✓</button>
                    {:else if video.status === 'ignored'}
                        <button class="btn-secondary" onclick={handleUnignore} disabled={actionWorking}>
                            Unignore
                        </button>
                    {/if}
                </div>
                {#if actionMsg}<p class="action-msg">{actionMsg}</p>{/if}
                <a
                    class="yt-link"
                    href="https://www.youtube.com/watch?v={video.youtube_id}"
                    target="_blank"
                    rel="noreferrer"
                >Watch on YouTube ↗</a>
            </div>
        </div>

        {#if video.description}
            <section class="description">
                <h2>Description</h2>
                <pre class="desc-text">{video.description}</pre>
            </section>
        {:else}
            <p class="loading">Loading description…</p>
        {/if}
    {/if}
</main>

<style>
    main { max-width: 860px; padding: 1rem; font-family: sans-serif; }
    .back { color: #666; text-decoration: none; font-size: 0.85rem; display: block; margin-bottom: 1rem; }
    .back:hover { color: #ccc; }
    .loading { color: #888; font-style: italic; }
    .error { color: #f44; }

    .detail { display: flex; gap: 1.5rem; margin-bottom: 1.5rem; flex-wrap: wrap; }
    .thumb { width: 320px; max-width: 100%; border-radius: 6px; flex-shrink: 0; background: #1e1e2e; }
    .info { flex: 1; min-width: 200px; }
    .title { font-size: 1.15rem; font-weight: 600; color: #ddd; margin: 0 0 0.4rem; line-height: 1.3; }
    .meta { font-size: 0.8rem; color: #666; margin: 0 0 0.5rem; }
    .status { font-weight: 600; font-size: 0.9rem; margin: 0 0 0.75rem; }
    .actions { display: flex; gap: 0.5rem; flex-wrap: wrap; margin-bottom: 0.5rem; }
    .btn-primary { background: #4af; color: #000; border: none; border-radius: 4px; padding: 0.4rem 1rem; font-size: 0.9rem; font-weight: 600; cursor: pointer; }
    .btn-secondary { background: #333; color: #aaa; border: 1px solid #555; border-radius: 4px; padding: 0.4rem 1rem; font-size: 0.9rem; cursor: pointer; }
    .btn-disabled { background: #222; color: #666; border: 1px solid #444; border-radius: 4px; padding: 0.4rem 1rem; font-size: 0.9rem; cursor: default; }
    .btn-disabled.downloaded { color: #4c4; border-color: #4c4; }
    .action-msg { font-size: 0.8rem; color: #4c4; margin: 0; }
    .yt-link { color: #4af; font-size: 0.8rem; text-decoration: none; }
    .yt-link:hover { text-decoration: underline; }

    .description { border-top: 1px solid #333; padding-top: 1rem; }
    .description h2 { font-size: 1rem; color: #bbb; margin: 0 0 0.5rem; }
    .desc-text {
        white-space: pre-wrap;
        word-break: break-word;
        font-family: sans-serif;
        font-size: 0.85rem;
        color: #aaa;
        line-height: 1.6;
        margin: 0;
        max-height: 400px;
        overflow-y: auto;
    }
</style>
```

- [ ] **Step 2: Build**

```bash
cd /home/jamiet/code/yt-plex/web && pnpm build 2>&1 | tail -5
```

Expected: `✔ done`

- [ ] **Step 3: Run full Rust test suite**

```bash
cd /home/jamiet/code/yt-plex && cargo test 2>&1 | grep -E "^test result|FAILED"
```

Expected: all pass.

- [ ] **Step 4: Commit**

```bash
cd /home/jamiet/code/yt-plex
git add web/src/routes/browse/[channelId]/[videoId]/+page.svelte
git commit -m "feat: video detail page with description, actions, YouTube link"
```

---

## Self-Review

**Spec coverage:**

| Feature | Covered by |
|---|---|
| FTS search by title and description | Task 2 (schema), Task 3 (query), Task 5 (route), Task 6 (API type), Task 7 (search input) |
| Infinite scroll instead of loading all at once | Task 3 (limit/offset in DB), Task 5 (route params), Task 6 (API types), Task 7 (IntersectionObserver) |
| Video detail page with description | Task 1 (model), Task 4 (fetch_video_description), Task 5 (GET endpoint), Task 6 (getVideo), Task 8 (page) |
| Multi-select bulk download / ignore | Task 7 (checkbox, bulk-bar, bulkDownload, bulkIgnore) |

**Type consistency check:**

- `VideoPage` defined in `models.rs` Task 1, used in `db.rs` Task 3, serialized in route Task 5, `VideoPage` interface in `api.ts` Task 6 — consistent.
- `list_videos_for_channel(channel_id, filter, show_ignored, search, limit, offset) -> Result<VideoPage>` — signature used identically in Task 3 (impl), Task 3 (tests), Task 5 (route call).
- `getVideo` in `api.ts` calls `GET /api/videos/{youtube_id}` — route added in Task 5, handler in `videos.rs`.
- `fetch_video_description(youtube_id) -> Result<String>` — defined in Task 4, called in Task 5 — consistent.
- `Video.description: Option<String>` / `description: string | null` — consistent across Rust and TypeScript.

**Placeholder scan:** No TBDs, no "similar to Task N", all steps have code. ✓

**Migration note:** The `videos` table gains a `description TEXT` column. Since `CREATE TABLE IF NOT EXISTS` won't add it to an existing table, delete the dev database (`~/.local/share/yt-plex/db.sqlite`) before the first run after Task 2.
