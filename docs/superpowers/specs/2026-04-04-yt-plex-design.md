# yt-plex Design Spec

**Date:** 2026-04-04

## Overview

`yt-plex` is a web server that lets an admin submit YouTube URLs for download via yt-dlp, automatically copies downloaded files to a Plex media server using configurable path templates, and triggers a Plex library scan after each download. A SvelteKit UI is embedded in the binary.

## Architecture

Single Rust binary using a Cargo workspace:

- **`crates/server`** — Axum web server, routes, background worker, WebSocket hub
- **`crates/common`** — config types, shared structs
- **`web/`** — SvelteKit frontend, built and embedded via rust-embed at compile time

Config is loaded from a TOML file at startup (XDG path: `~/.config/yt-plex/config.toml`).

## Data Model

SQLite database with two tables.

### `jobs`

| Column | Type | Notes |
|---|---|---|
| `id` | TEXT | UUID v4 |
| `url` | TEXT | YouTube URL submitted |
| `status` | TEXT | `queued`, `downloading`, `copying`, `done`, `failed` |
| `channel_name` | TEXT | Populated from yt-dlp output, nullable |
| `title` | TEXT | Populated from yt-dlp output, nullable |
| `error` | TEXT | Error message if failed, nullable |
| `created_at` | TEXT | ISO 8601 |
| `updated_at` | TEXT | ISO 8601 |

### `sessions`

| Column | Type | Notes |
|---|---|---|
| `token` | TEXT | Random 32-byte hex token |
| `created_at` | TEXT | ISO 8601 |
| `expires_at` | TEXT | ISO 8601, 7-day expiry |

## Configuration (TOML)

```toml
[server]
bind = "0.0.0.0:3000"

[auth]
admin_password_hash = "..."  # argon2 hash, generated via `yt-plex hash-password`

[plex]
url = "http://192.168.1.x:32400"
token = "..."
library_section_id = "..."  # which library section to refresh

[output]
base_path = "/mnt/plex/YouTube"
path_template = "{channel}/{date} - {title}.{ext}"
```

Available template variables: `{channel}`, `{title}`, `{date}` (YYYY-MM-DD of download), `{ext}`.

A `yt-plex hash-password` subcommand generates an argon2 hash from a plaintext password for use in config.

## Auth

Single admin account. Password stored as an argon2 hash in config. On login, server issues a random 32-byte hex session token stored in the `sessions` table with a 7-day expiry, sent as an `HttpOnly` cookie. No registration flow — admin password is set via CLI only.

Unauthenticated requests can view the job list. Submitting URLs and accessing settings requires a valid session cookie.

## Background Worker

A single Tokio task runs the download loop:

1. Poll SQLite every 5 seconds for `status = 'queued'` jobs (oldest first)
2. Set status → `downloading`; broadcast status change over WebSocket
3. Spawn `yt-dlp --print-json <url> -o <tempdir>/%(id)s.%(ext)s` via `tokio::process::Command`; capture stdout to extract `channel`, `title`, `ext`, and output filename
4. Apply path template to determine destination path under `base_path`; create parent directories as needed
5. Move (rename) file from temp dir to destination; set status → `copying`; broadcast
6. On success: set status → `done`; call Plex API (`GET /library/sections/{id}/refresh`); broadcast
7. On any failure: set status → `failed` with error message; broadcast

One job runs at a time (sequential, no parallel downloads) for MVP.

## WebSocket

A single `/ws` endpoint. Connected clients receive JSON messages when any job's status changes:

```json
{ "job_id": "...", "status": "downloading", "channel": "...", "title": "..." }
```

No authentication required to connect to `/ws` (read-only status updates).

## Web UI

SvelteKit app, built to `web/build/` and embedded in the binary via rust-embed.

| Route | Auth required | Description |
|---|---|---|
| `/login` | No | Admin password form |
| `/` | No (read-only) | Job list with live status; URL submission form for admins |
| `/settings` | Admin | View/edit Plex URL, token, path template, base path |

The job list connects to `/ws` on load and updates in real time.

## Post-MVP (out of scope for this spec)

- Configure allowed YouTube channels; sync all videos from a channel
- Regular user accounts (non-admin) who can browse channel videos and request downloads
- Parallel downloads
- Download progress percentage (via yt-dlp `--progress` output parsing)
