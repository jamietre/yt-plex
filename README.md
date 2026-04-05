# yt-plex

Download YouTube videos via yt-dlp and sync them to your Plex media server.

## Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [mise](https://mise.jdx.dev/) (manages Node.js and pnpm versions)
- [yt-dlp](https://github.com/yt-dlp/yt-dlp) on your `PATH` — install on Debian/Ubuntu:
  ```bash
  curl -L https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp -o ~/.local/bin/yt-dlp
  chmod a+rx ~/.local/bin/yt-dlp
  ```
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

### 3. Create Google OAuth credentials

Go to [console.cloud.google.com](https://console.cloud.google.com) → **APIs & Services** → **Credentials** → **Create OAuth 2.0 Client ID**.

Choose type **"TVs and Limited Input devices"**. No redirect URIs are needed (the app uses device flow and does not need to be publicly accessible).

Copy the **Client ID** and **Client Secret** for the config below.

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
client_id = "your-client-id.apps.googleusercontent.com"
client_secret = "your-client-secret"

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

### 5. Run

```bash
./target/release/yt-plex
# or with a custom config path:
./target/release/yt-plex --config /path/to/config.toml
```

Open `http://localhost:3000` in your browser.

## Usage

1. Click **Sign in with Google**. You'll see a short code and a URL.
2. On any device, open the URL and enter the code to authenticate with your Google account.
3. Once authorised, the browser redirects you to the dashboard automatically.
4. Paste a YouTube URL into the input box and click **Add**.
5. The server downloads the video in the background, copies it to your Plex path, and triggers a library scan.
6. Job status updates live in the browser via WebSocket.

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
