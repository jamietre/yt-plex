# yt-plex

Download YouTube videos via yt-dlp and organise them as a Plex TV Shows library, with full metadata from the [YouTube Agent for Plex](https://github.com/jamietre/YouTube-Agent.bundle).

## How it works

1. You add YouTube channels. yt-plex scrapes their video lists via yt-dlp.
2. You (or household members via profiles) browse and queue downloads.
3. yt-dlp downloads the video and writes a `.info.json` sidecar file alongside it.
4. The file lands in a folder structure compatible with Absolute Series Scanner + YouTube Agent.
5. Plex picks it up, reads the sidecar for metadata (title, description, thumbnail, date) — no YouTube API key required.

---

## Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [mise](https://mise.jdx.dev/) (manages Node.js and pnpm versions)
- [yt-dlp](https://github.com/yt-dlp/yt-dlp) on your `PATH`:
  ```bash
  curl -L https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp -o ~/.local/bin/yt-dlp
  chmod a+rx ~/.local/bin/yt-dlp
  ```
- A running Plex Media Server

---

## Setup

### 1. Install tooling

```bash
mise install
```

### 2. Build

```bash
mise run build
```

Binary lands at `target/release/yt-plex`.

> **Without mise:**
> ```bash
> cd web && pnpm install && pnpm build && cd ..
> cargo build --release -p yt-plex-server
> ```

### 3. Create Google OAuth credentials

Go to [console.cloud.google.com](https://console.cloud.google.com) → **APIs & Services** → **Credentials** → **Create OAuth 2.0 Client ID**.

Choose type **"TVs and Limited Input devices"**. No redirect URI needed (device flow).

Copy the **Client ID** and **Client Secret** for the config.

### 4. Create a config file

```bash
mkdir -p ~/.config/yt-plex
```

Create `~/.config/yt-plex/config.toml`:

```toml
[server]
bind = "0.0.0.0:3000"

[auth]
admin_emails = ["you@gmail.com"]

[google_oauth]
client_id     = "your-client-id.apps.googleusercontent.com"
client_secret = "your-client-secret"

[plex]
url                = "http://192.168.1.x:32400"
token              = "your-plex-token"
library_section_id = "1"          # Admin → Settings → fetch from Plex

[output]
base_path     = "/mnt/plex/YouTube"
path_template = "{channel} [{channel_id}]/Season {yyyy}/[{date}] - {title} [{id}].{ext}"

[sync]
interval_hours = 6
playlist_items = 50
```

> **`path_template`** — the format above is required for Plex + YouTube Agent integration. See [Plex Integration](#plex-integration) below.

**Finding your Plex token:** Plex Web → any item → `···` → Get Info → View XML → copy `X-Plex-Token=` from the URL. Or see [Plex support](https://support.plex.tv/articles/204059436/).

**Finding your library section ID:** Open Admin → Settings in yt-plex and click **Fetch from Plex**.

### 5. Run

```bash
./target/release/yt-plex
# custom config path:
./target/release/yt-plex --config /path/to/config.toml
```

Open `http://localhost:3000`.

---

## Plex Integration

yt-plex is designed to work with **Absolute Series Scanner** and the **YouTube Agent** so that each channel appears as a TV show in Plex with proper episode metadata, thumbnails, and descriptions — sourced from `.info.json` sidecar files written by yt-dlp (no YouTube API key required).

### Plex library setup

Create a **TV Shows** library in Plex pointed at your `base_path`. Configure it as:

| Setting | Value |
|---------|-------|
| Library type | **TV Shows** |
| Scanner | **Absolute Series Scanner** |
| Agent | **YouTube Agent** (Series) |

### Installing Absolute Series Scanner

Download and place it in your Plex Scanners directory:

```bash
# Linux (adjust path for your Plex install)
curl -fsSL "https://raw.githubusercontent.com/ZeroQI/Absolute-Series-Scanner/master/Scanners/Series/Absolute%20Series%20Scanner.py" \
  -o "/var/lib/plexmediaserver/Library/Application Support/Plex Media Server/Scanners/Series/Absolute Series Scanner.py"
```

Restart Plex after installing.

### Installing YouTube Agent

```bash
cd "/var/lib/plexmediaserver/Library/Application Support/Plex Media Server/Plug-ins"
git clone https://github.com/jamietre/YouTube-Agent.bundle.git
```

Restart Plex after installing.

### Path template

The recommended template:

```
{channel} [{channel_id}]/Season {yyyy}/[{date}] - {title} [{id}].{ext}
```

This produces a layout like:

```
Veritasium [UCHnyfMqiRRz1biKb2GHbUuQ]/
  Season 2024/
    [2024-03-15] - Why Does Anything Exist [wupToqz1e2g].mkv
    [2024-03-15] - Why Does Anything Exist [wupToqz1e2g].info.json
```

The elements the scanner and agent depend on:

- **`[{channel_id}]`** in the folder name — tells the agent which YouTube channel this show belongs to (e.g. `[UCHnyfMqiRRz1biKb2GHbUuQ]`)
- **`[{id}]`** in the filename — the YouTube video ID, used to look up metadata
- **Season folders** — `Season {yyyy}` groups episodes by upload year
- **`.info.json` sidecar** — written automatically by yt-dlp; the agent reads this for title, description, thumbnail, upload date, and duration without hitting the YouTube API

### Metadata without a YouTube API key

yt-plex passes `--write-info-json` to yt-dlp on every download. The resulting `.info.json` sidecar is placed alongside the video file. The YouTube Agent reads this file first, so Plex gets full metadata even with no internet access and no API key configured in the agent.

If you want the agent to also fetch metadata for pre-existing files (without `.info.json`), configure a YouTube Data API v3 key in the agent's preferences in Plex.

---

## Configuration reference

### `[output]`

| Key | Description |
|-----|-------------|
| `base_path` | Root directory where videos are written |
| `path_template` | Relative path template for each video (see variables below) |
| `thumbnail_cache_dir` | Where yt-plex caches YouTube thumbnails for its own UI |

**Template variables:** `{channel}`, `{channel_id}`, `{title}`, `{id}`, `{ext}`, `{date}` (YYYY-MM-DD), `{yyyy}`, `{mm}`, `{dd}`

> The YouTube video ID must appear wrapped in square brackets somewhere in the filename: `[{id}]`. This is how yt-plex matches downloaded files back to their database record, and how the YouTube Agent identifies episodes.

### `[sync]`

| Key | Default | Description |
|-----|---------|-------------|
| `interval_hours` | `6` | How often yt-plex auto-syncs all channels |
| `playlist_items` | `50` | Number of recent videos to fetch per channel sync |

### `[download]`

| Key | Default | Description |
|-----|---------|-------------|
| `extra_args` | `[]` | Extra arguments passed verbatim to yt-dlp before the URL |

Example — use a cookies file to access age-restricted or member-only content:

```toml
[download]
extra_args = [
  "--cookies", "/config/cookies.txt",
  "--rate-limit", "2M",
]
```

---

## Development

```bash
mise run dev          # backend + frontend concurrently
mise run dev-server   # backend only (cargo-watch)
mise run web-dev      # frontend only (proxies /api/* to :3000)
cargo test
```

---

## Notes

- One download runs at a time (sequential queue).
- Plex library refresh is triggered after each successful download.
- If Plex is unreachable, the download still completes and a warning is logged.
- The SQLite database lives at `~/.local/share/yt-plex/db.sqlite` by default.
