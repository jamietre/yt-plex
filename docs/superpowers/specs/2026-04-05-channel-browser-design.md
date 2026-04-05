# Channel Browser Design Spec

**Date:** 2026-04-05

## Overview

Extends yt-plex with a channel browser: admins maintain a list of allowed YouTube channels; any user on the local network can browse those channels, see which videos are new vs already on Plex, download videos with one click, and ignore videos they don't want to see again.

The arbitrary URL submission feature (current admin homepage) becomes admin-only. Regular users interact exclusively through the channel browser.

---

## User Roles

| Role | Auth | Capabilities |
|---|---|---|
| Regular user | None (anonymous) | Browse channels, view video status, download from approved channels, ignore videos |
| Admin | Google OAuth | All of the above, plus: manage channel list, submit arbitrary URLs, access Plex/output settings |

This is a local-network app; no login is required for regular users. The ignore state is global (stored in SQLite) — shared across all users. Per-user ignore lists may be added later when user accounts are introduced.

---

## Navigation

Tabbed layout replacing the current single-page design:

| Tab | Path | Visible to |
|---|---|---|
| Browse | `/browse` | Everyone |
| Queue | `/queue` | Everyone |
| Admin | `/admin` | Admins only (hidden otherwise) |

`/` redirects to `/browse`.

---

## Data Model

Three new tables added to the existing SQLite database.

### `channels`

| Column | Type | Notes |
|---|---|---|
| `id` | TEXT | UUID v4 |
| `youtube_channel_url` | TEXT | URL as entered by admin |
| `name` | TEXT | Display name populated on first sync |
| `last_synced_at` | TEXT | ISO 8601, nullable |

### `videos`

| Column | Type | Notes |
|---|---|---|
| `youtube_id` | TEXT | Primary key (e.g. `dQw4w9WgXcQ`) |
| `channel_id` | TEXT | FK → channels.id |
| `title` | TEXT | |
| `published_at` | TEXT | ISO 8601, nullable |
| `downloaded_at` | TEXT | ISO 8601, nullable — set by filesystem scan or on job completion |
| `last_seen_at` | TEXT | ISO 8601 — last sync that returned this video |
| `ignored_at` | TEXT | ISO 8601, nullable — set when user ignores |

### No schema changes to `jobs` or `sessions`.

### Derived video status

Status is not stored; it is computed per video at query time:

1. Active job (status `queued`/`downloading`/`copying`) for this `youtube_id` → **in progress** (treated as downloaded for filter purposes)
2. `downloaded_at IS NOT NULL` → **downloaded**
3. `ignored_at IS NOT NULL` → **ignored**
4. Otherwise → **new**

Filter behaviour:
- **New** (default): status = new only
- **Downloaded**: status = downloaded or in progress
- **All**: everything except ignored
- **Show ignored toggle**: when on, ignored videos appear in whichever filter is active

---

## Channel Sync

yt-dlp with `--flat-playlist --print "%(id)s\t%(title)s\t%(upload_date)s"` — no YouTube API key required.

### First sync (channel newly added, `last_synced_at IS NULL`)

Full channel scan with no playlist limit. Fetches the complete back catalogue and populates `videos`. Triggered immediately when admin adds a channel.

### Routine sync

Fetches only the most recent N videos per channel (`--playlist-items 1:N`, default N=50, configurable). Runs on a background timer cycling through all channels.

### Manual refresh

Any user can trigger a per-channel sync from the channel video page. Behaves identically to a routine sync (last N videos).

### Filesystem scan

After each sync (first or routine), the sync job scans `base_path` recursively for filenames matching `[youtube_id]` (bracket suffix) and sets `downloaded_at` on matching `videos` rows. `downloaded_at` is also set immediately on job completion without waiting for the next scan.

### Config

```toml
[sync]
interval_hours = 6    # background sync frequency (default: 6)
playlist_items = 50   # videos fetched per routine sync (0 = unlimited)
```

---

## File Naming

The `{id}` template variable is added, carrying the YouTube video ID. The default path template is updated:

```toml
[output]
path_template = "{channel}/{date} - {title} [{id}].{ext}"
```

This embeds the video ID in the filename (e.g. `Veritasium/2026-04-05 - Why Black Holes Are Dark [dQw4w9WgXcQ].mp4`), enabling reliable cross-referencing during filesystem scans regardless of title changes or renames — as long as the `[id]` suffix is preserved.

**Migration note:** Files downloaded before this change lack the bracket suffix and will not be recognised as downloaded by the filesystem scan. Given the app is under initial development, this is an accepted trade-off.

---

## API Routes

### New routes

| Method | Path | Auth | Description |
|---|---|---|---|
| `GET` | `/api/channels` | None | List all channels with `last_synced_at` |
| `POST` | `/api/channels` | Admin | Add channel; triggers first sync immediately |
| `DELETE` | `/api/channels/:id` | Admin | Remove channel and its videos |
| `POST` | `/api/channels/:id/sync` | None | Trigger manual sync for one channel |
| `GET` | `/api/channels/:id/videos` | None | List videos with derived status. Query params: `filter=new\|downloaded\|all`, `show_ignored=true` |
| `POST` | `/api/videos/:youtube_id/ignore` | None | Mark video as ignored |
| `DELETE` | `/api/videos/:youtube_id/ignore` | None | Un-ignore video |

### Modified routes

**`POST /api/jobs`** — now accepts two forms:
- `{ "url": "..." }` — admin only (arbitrary URL, existing behaviour)
- `{ "youtube_id": "..." }` — open to all; constructs `https://www.youtube.com/watch?v={id}` and queues download. Only valid if the `youtube_id` exists in the `videos` table (i.e. belongs to an approved channel).

---

## Frontend

### New / modified SvelteKit routes

| Route | Description |
|---|---|
| `/` | Redirects to `/browse` |
| `/browse` | Channel grid — all allowed channels with name and last-synced time |
| `/browse/[channelId]` | Video grid for one channel. Filter pills (New / Downloaded / All), Show Ignored toggle, per-video status badge, Download and Ignore buttons |
| `/queue` | Job list with live WebSocket updates (moved from `/`) |
| `/login` | Unchanged |
| `/admin` | Replaces `/settings` — Plex config, output config, channel management (add/remove), arbitrary URL submission |

### Video card

Each card shows:
- YouTube thumbnail (`https://img.youtube.com/vi/{id}/mqdefault.jpg` — no proxy needed)
- Title
- Status badge: **NEW** / **ON PLEX** / **↓ N%** (downloading) / **QUEUED**
- **Download** button (new), **On Plex ✓** (downloaded, disabled), or progress indicator (in progress)
- **Ignore** button (hidden while in progress or already downloaded)

### Real-time updates

The existing WebSocket broadcast (`WsMessage`) will include `youtube_id` so the video grid can update status badges live when a download starts or completes — no page refresh required.

---

## WsMessage change

Add `youtube_id: Option<String>` to `WsMessage` (Rust) and `youtube_id?: string` to the TypeScript interface. Set when the job was queued via `youtube_id` submission. The video grid subscribes to the store and updates the matching card.

---

## Out of Scope

- Per-user ignore lists (deferred until user accounts are introduced)
- Parallel downloads
- Pagination of video grid (can add later if channels have large catalogues)
- YouTube API key support (yt-dlp flat-playlist covers the use case without one)
