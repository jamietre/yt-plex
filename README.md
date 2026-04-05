# yt-plex

Download YouTube videos via yt-dlp and sync them to your Plex media server.

## Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [mise](https://mise.jdx.dev/) (manages Node.js and pnpm versions)
- [yt-dlp](https://github.com/yt-dlp/yt-dlp) on your `PATH`
- A running Plex Media Server

## Setup

### 1. Install Node.js tooling via mise

```bash
mise install
```

### 2. Build everything

```bash
mise run build
```

This installs web dependencies, builds the SvelteKit UI, then compiles the Rust binary.

The binary lands at `target/release/yt-plex`.

> **Manual steps (if not using mise):**
> ```bash
> cd web && pnpm install && pnpm build && cd ..
> cargo build --release -p yt-plex-server
> ```

### 3. Create a config file

```bash
mkdir -p ~/.config/yt-plex
```

Generate a password hash:

```bash
./target/release/yt-plex hash-password YOUR_PASSWORD
```

Create `~/.config/yt-plex/config.toml`:

```toml
[server]
bind = "0.0.0.0:3000"

[auth]
admin_password_hash = "<output from hash-password above>"

[plex]
url = "http://192.168.1.x:32400"
token = "your-plex-token"
library_section_id = "1"          # find in Plex → Settings → Libraries

[output]
base_path = "/mnt/plex/YouTube"
path_template = "{channel}/{date} - {title}.{ext}"
```

**Finding your Plex token:** Settings → Remote Access → "Show Advanced" → copy the token from any API URL, or see [Plex support](https://support.plex.tv/articles/204059436/).

**Finding your library section ID:** Browse to `http://<plex>:32400/library/sections?X-Plex-Token=<token>` and find the `key` for your YouTube library.

**Path template variables:** `{channel}`, `{title}`, `{date}` (YYYY-MM-DD), `{ext}`

### 4. Run

```bash
./target/release/yt-plex
# or with a custom config path:
./target/release/yt-plex --config /path/to/config.toml
```

Open `http://localhost:3000` in your browser.

## Usage

1. Log in with the admin password you configured.
2. Paste a YouTube URL into the input box and click **Add**.
3. The server downloads the video in the background, copies it to your Plex path, and triggers a library scan.
4. Job status updates live in the browser via WebSocket.

## Development

```bash
# Run backend (watches for changes with cargo-watch):
mise run dev-server

# Run frontend dev server (proxies API to backend):
mise run web-dev
```

## Notes

- One download runs at a time (sequential queue).
- If a download fails, the error is shown in the job list.
- Plex library refresh is triggered after each successful download; if your Plex server is unreachable the download still completes and a warning is logged.
