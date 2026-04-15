# Docker Deployment Guide

## Prerequisites

- Docker and Docker Compose on the target host
- Google OAuth credentials (Client ID + Secret) — see [Google OAuth setup](#google-oauth-setup)
- A Plex Media Server accessible from the target host

---

## Installation (standalone)

This procedure works on any host without needing the dev repo.

### 1. Create the working directory

```bash
mkdir -p ~/yt-plex/config
cd ~/yt-plex
```

### 2. Write `docker-compose.yml`

```yaml
services:
  yt-plex:
    image: yt-plex:latest
    restart: unless-stopped
    ports:
      - "3000:3000"
    volumes:
      - ./config:/config       # config.toml
      - ./data:/data           # SQLite DB + thumbnail cache
      - /mnt/media2:/media     # Plex media directory — adjust host path as needed

```

### 3. Write `config/config.toml`

Copy from `config.toml.example` in the repo, or start from this template:

```toml
[server]
bind = "0.0.0.0:3000"

[auth]
admin_emails = ["you@gmail.com"]

[google_oauth]
client_id     = "YOUR_CLIENT_ID.apps.googleusercontent.com"
client_secret = "YOUR_CLIENT_SECRET"

[plex]
url                = "http://your-plex-host:32400"
token              = "YOUR_PLEX_TOKEN"
library_section_id = ""            # leave blank; use Admin → Settings → Fetch from Plex

[output]
base_path           = "/media/video/youtube"
path_template       = "{channel} [{channel_id}]/Season {yyyy}/[{date}] - {title} [{id}].{ext}"
thumbnail_cache_dir = "/data/thumbnails"

[sync]
interval_hours = 6
playlist_items = 50
```

> See the [Plex integration section in the README](../README.md#plex-integration) for how `path_template` works with Absolute Series Scanner and YouTube Agent.

### 4. Load the Docker image

Either build it yourself from the repo root:

```bash
docker build -t yt-plex:latest .
```

Or receive it from someone who built it:

```bash
docker load < yt-plex.tar.gz
```

### 5. Start

```bash
cd ~/yt-plex
docker compose up -d
docker compose logs -f    # should print "Listening on 0.0.0.0:3000"
```

Open `http://<host>:3000`.

---

## Updating

```bash
# Load the new image (built elsewhere or built locally)
docker load < yt-plex.tar.gz

cd ~/yt-plex
docker compose up -d      # restarts with new image; DB migrations run automatically
```

---

## Developer deploy workflow

If you have the repo locally, use the mise tasks:

```bash
mise run docker-build          # build image
mise run docker-push-config    # copy config.toml + docker-compose.yml to $DEPLOY_HOST
mise run docker-deploy         # ship image + restart container
mise run docker-logs           # tail logs
```

`DEPLOY_HOST` must be set in `.env` (e.g. `DEPLOY_HOST=root@192.168.1.100`). Override on the command line:

```bash
DEPLOY_HOST=root@otherhost mise run docker-deploy
```

---

## Finding your Plex token

Plex Web → any media item → `···` → Get Info → View XML → copy `X-Plex-Token=` from the URL.

---

## Google OAuth setup

1. Go to [Google Cloud Console](https://console.cloud.google.com/) → APIs & Services → Credentials.
2. Create an OAuth 2.0 Client ID → choose **TV and Limited Input devices**.
3. Copy the Client ID and Client Secret into `config.toml`.
4. No redirect URI needed — yt-plex uses the device flow.

---

## Volume reference

| Mount | Purpose |
|-------|---------|
| `./config:/config` | `config.toml` |
| `./data:/data` | SQLite DB + thumbnail cache (persists across restarts) |
| `/mnt/media2:/media` | Plex media output directory (adjust host path) |

---

## Useful commands

```bash
docker compose logs -f yt-plex                   # live logs
docker compose exec yt-plex bash                 # shell inside container
docker compose down                              # stop (data preserved)
docker compose down -v                           # stop + wipe volumes (destructive)

# Update yt-dlp inside the running container
docker compose exec yt-plex \
  curl -fsSL https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp \
  -o /usr/local/bin/yt-dlp
```
