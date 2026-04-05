# yt-plex — Project Guide for Claude

## What this is

yt-plex is a home-network YouTube-to-Plex downloader. It scrapes video metadata from approved YouTube channels via yt-dlp, lets household members browse and queue downloads, and organises the output for a Plex TV Shows library.

---

## Repository layout

```
Cargo.toml              workspace root
crates/
  common/               shared types (models, config) — no server deps
    src/models.rs       Job, Channel, Video, Profile, VideoPage, WsMessage, …
    src/config.rs       Config struct, TOML load/save
  server/               Axum web server + all business logic
    src/main.rs         entry point (clap, config load, server start)
    src/lib.rs          AppState, build_router(), create_app_state()
    src/db.rs           SQLite via rusqlite — all DB operations + migrations
    src/sync.rs         yt-dlp flat-playlist scraping, filesystem scanner
    src/worker.rs       background download queue worker
    src/ws.rs           WebSocket broadcast hub
    src/auth.rs         token generation helpers
    src/template.rs     Plex path template renderer ({channel}, {title}, …)
    src/plex.rs         Plex library refresh via HTTP
    src/routes/
      auth.rs           Google OAuth device flow, session cookie, /api/auth/*
      channels.rs       channel CRUD + per-channel sync, /api/channels/*
      videos.rs         video detail + per-profile ignore/unignore
      jobs.rs           download queue submission + listing
      profiles.rs       profile CRUD, session cookie, channel subscriptions
      settings.rs       config read/write, /api/settings
      thumbnails.rs     local-caching YouTube thumbnail proxy
      assets.rs         rust-embed static asset fallback (serves SvelteKit SPA)
web/                    SvelteKit 5 SPA (adapter-static, SPA mode)
  src/lib/api.ts        all fetch wrappers + TypeScript types
  src/routes/           file-based routing
    +layout.svelte      nav, profile guard (redirects to /select-profile)
    browse/+page.svelte channel grid with subscription toggles
    browse/[channelId]/+page.svelte         video grid (search, infinite scroll, bulk)
    browse/[channelId]/[videoId]/+page.svelte  video detail page
    select-profile/+page.svelte  profile picker (first visit)
    queue/+page.svelte  download queue with WebSocket live updates
    admin/+page.svelte  channels, profiles, URL submission, settings
    login/+page.svelte  Google device flow login
```

---

## Key architecture decisions

### Single binary
The SvelteKit build output is embedded into the Rust binary at compile time via `rust-embed`. The server serves the SPA from `/` and falls back to `index.html` for client-side routes.

### Auth model
- One admin account — Google email(s) listed in `config.toml` under `[auth] admin_emails`.
- Login via Google OAuth device flow. On success, a `session` cookie (HttpOnly, 7-day) is set.
- User profiles are selected separately via a `yt_plex_profile` cookie (NOT HttpOnly — JS-readable for display).
- Admin profiles are auto-created/linked on first OAuth login and hidden from the public profile picker.

### User profiles
- Non-admin users pick a profile on first visit (`/select-profile`).
- Each profile has its own channel subscriptions (`profile_channels`) and ignore list (`profile_video_ignores`).
- `videos.ignored_at` is a global admin-level suppress; per-user ignores are in `profile_video_ignores`.
- Admin bypasses profile filtering and sees all channels/videos.

### Axum version
Uses **Axum 0.8**. Route parameters use `{param}` syntax (not `:param`).
Custom extractors: `SessionToken` (reads `session=` cookie), `ProfileCookie` (reads `yt_plex_profile=` cookie).

### Download pipeline
1. `POST /api/jobs` queues a job in SQLite.
2. `worker.rs` polls for queued jobs, runs `yt-dlp`, updates status, broadcasts via WebSocket.
3. On completion, `sync::scan_filesystem` matches downloaded files by `[youtube_id]` suffix and records `file_path`.

---

## Development

```bash
mise run dev          # backend (cargo watch) + frontend (vite dev) concurrently
mise run dev-server   # backend only
mise run web-dev      # frontend only (proxies /api/* to :3000)
cargo test            # all tests
mise run db-reset     # delete local SQLite DB (forces re-creation on next start)
```

Config is read from `~/.config/yt-plex/config.toml` by default (override with `$YT_PLEX_CONFIG`).

---

## Database migrations

The database schema is versioned via `PRAGMA user_version`. Migrations live in
`crates/server/src/db.rs` as the `MIGRATIONS` constant.

### Rules — follow these strictly

1. **Never edit an existing migration.** Index 0 is v1, index 1 is v2, etc. Once committed, a migration is immutable.
2. **Always add a new migration for schema changes.** Append to the `MIGRATIONS` slice.
3. **Use `ALTER TABLE … ADD COLUMN` for additive column additions** — do not rewrite the original `CREATE TABLE`.
4. Each migration runs inside `run_migrations()` which applies versions sequentially and bumps `user_version` after each one.

### Example — adding a column

```rust
// In MIGRATIONS, append:
// ── v2: add thumbnail_url to channels ────────────────────────────────────────
"ALTER TABLE channels ADD COLUMN thumbnail_url TEXT;",
```

Never backfill migration v1 with the new column. The `ALTER TABLE` in v2 handles both new and existing databases.

---

## Deployment (Docker)

```bash
docker build -t yt-plex:latest .
docker compose up -d
```

See `docker-compose.yml` for volume mounts: `/config` (config.toml), `/data` (SQLite + thumbnail cache), `/media` (Plex output directory).
