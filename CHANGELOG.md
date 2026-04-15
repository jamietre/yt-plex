# Changelog

## [Unreleased] — feat/channel-browser

### Changed
- **Deploy scripts** — removed hardcoded private IP from `mise.toml`, `scripts/komodo-deploy.sh`, `docker-compose.yml`, and docs; `REGISTRY`, `DEPLOY_HOST`, `KOMODO_HOST` must now be set in `.env` (scripts fail with a clear error if missing)

### Added
- Video detail page (`/browse/[channelId]/[videoId]`) with thumbnail, description, status, file path, and actions
- Search input on channel browse page (debounced, FTS5 title-only)
- Infinite scroll on channel browse page (IntersectionObserver)
- Multi-select checkboxes with bulk download / bulk ignore actions
- `GET /api/videos/{id}` endpoint with lazy description loading
- `GET /api/thumbnails/{youtube_id}` — local caching proxy for YouTube thumbnails (avoids direct client→YouTube requests)
- Configurable thumbnail cache directory (`output.thumbnail_cache_dir` in config.toml)
- Sync descriptions for newly-seen videos via `yt-dlp -j`
- FTS5 full-text search index on `videos.title` and `videos.description`
- `{yyyy}`, `{mm}`, `{dd}` path template variables extracted from upload date
- Show YouTube upload date on video detail page; fall back to "first seen" date when unavailable
- Show Plex file path on video detail page once downloaded
- Pre-populate job `channel_name` and `title` from DB when submitting via `youtube_id`
- Admin settings: explain path template variables and `[{id}]` requirement
- Thumbnail cache field in admin settings page
- Include `user_code` in Google device-auth verification URL so clicking the link auto-fills the code
- Sync status column in admin channel table with live polling
- `[tasks.db-reset]` in `mise.toml` for wiping the local SQLite database
- `Dockerfile` — multi-stage build (Node → Rust → Debian slim runtime) with ffmpeg and yt-dlp
- `docker-compose.yml` for simple server deployment

### Fixed
- **Auth** — switched OAuth flow to `authorization_code` with configurable `redirect_uri`; fixed `redirect_uri_mismatch` caused by wrong `client_id` in config
- **Theme contrast** — raised `--text-2`, `--text-3`, `--border`, and `--border-2` values for better readability of labels and inactive controls
- Infinite scroll was triggering dozens of duplicate requests due to stacked `IntersectionObserver` observations from a `$effect` re-running on every state change
- `published_at` was not updated on re-sync; fixed with `COALESCE` in upsert `ON CONFLICT` clause
- YouTube link on detail page is now admin-only
- Admin-only URL submission via `url` field still enforced; `youtube_id` path open to all users with an approved channel
- WAL/SHM files now included in `db-reset` task

### Changed
- **UI restyled** — cinematic dark theme applied across browse, queue, login, select-profile, and video detail pages
- Search restricted to `title` column only (not description) to reduce false positives
- `set_video_downloaded` now stores the file path alongside the downloaded timestamp
- `insert_job` now accepts optional `channel_name` and `title` for display in the queue before metadata is fetched
- `VideoFilter::from_str` renamed to `VideoFilter::parse` to avoid shadowing `std::str::FromStr`
- `sanitise` in `template.rs` collapses consecutive `replace` calls into a single char-set replace
- `WsHub` now derives `Default` via an explicit impl; `new()` delegates to it

### Added
- **Regen metadata** — new `POST /api/channels/{id}/regen-metadata` endpoint runs `yt-dlp --skip-download --write-info-json` for all downloaded videos; admin page exposes a "Regen metadata" button per channel
- **Docker registry deployment** — `Dockerfile` updated to multi-stage build targeting `rust:1.88`; `docker-compose.yml` wired to a local registry; `mise.toml` `docker-build` / `docker-deploy` tasks use Komodo API for redeployment
- **Config example** — `config.toml.example` documents all supported fields
- **Preferences page** — `/preferences` route for per-profile display settings (persisted in `localStorage` via `prefs.ts`)
- **Navigation search** — `navSearch.ts` store allows individual pages to register a search input in the nav bar
- **`favicon.svg`** — app icon
- **`scripts/komodo-deploy.sh`** — deploy helper for Komodo stack API
- `{channel_id}` path template variable — expands to the YouTube channel ID (e.g. `UCxxxxxxxxxxxxxxxx`)
- DB migration v2: `ALTER TABLE channels ADD COLUMN youtube_channel_id TEXT`
- Sync captures `%(channel_id)s` from yt-dlp flat-playlist and stores it on the channel record after first sync
- Admin channels table shows a "Channel ID" column (populated after sync)
- Admin settings help updated with `{channel_id}` variable, YouTube Agent plugin link, and recommended Plex TV Shows template
