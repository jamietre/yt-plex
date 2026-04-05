# Docker Deployment Guide

## Prerequisites

- Docker and Docker Compose installed on the server
- A Plex media directory the container can write to
- Google OAuth credentials (client ID + secret) from [Google Cloud Console](https://console.cloud.google.com/)

---

## First-time setup

### 1. Clone and configure

On your **server**, create a working directory and config folder:

```bash
mkdir -p ~/yt-plex/config
cd ~/yt-plex
```

Copy `docker-compose.yml` from the repo (or write it fresh):

```yaml
services:
  yt-plex:
    image: yt-plex:latest
    build: .
    restart: unless-stopped
    ports:
      - "3000:3000"
    volumes:
      - ./config:/config
      - yt-plex-data:/data
      - /path/to/plex/media:/media   # ← change this

volumes:
  yt-plex-data:
```

Create `config/config.toml`:

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
library_section_id = "1"           # find this in Plex → Settings → Libraries

[output]
base_path          = "/media"
path_template      = "{channel}/Season {yyyy}/{title} [{id}].{ext}"
thumbnail_cache_dir = "/data/thumbnails"

[database]
path = "/data/db.sqlite"

[sync]
interval_hours  = 6
playlist_items  = 50
```

### 2. Build the image

From the **repo root** (where `Dockerfile` lives):

```bash
docker build -t yt-plex:latest .
```

The build is multi-stage (Node → Rust → Debian slim) and takes a few minutes on first run. Subsequent builds are fast thanks to layer caching.

### 3. Ship to your server (if building locally)

```bash
docker save yt-plex:latest | gzip | ssh user@yourserver 'docker load'
```

Then copy the compose file and config:

```bash
scp docker-compose.yml user@yourserver:~/yt-plex/
scp -r config/         user@yourserver:~/yt-plex/
```

### 4. Start the container

```bash
cd ~/yt-plex
docker compose up -d
docker compose logs -f     # watch startup; should print "Listening on 0.0.0.0:3000"
```

---

## Updating to a new version

```bash
# On the build machine:
git pull
docker build -t yt-plex:latest .
docker save yt-plex:latest | gzip | ssh user@yourserver 'docker load'

# On the server:
cd ~/yt-plex
docker compose up -d       # restarts with the new image; DB migrations run automatically
```

The SQLite database lives in the `yt-plex-data` named volume — it is **not** removed by `docker compose up -d` or `docker compose down`. Only `docker compose down -v` deletes volumes.

---

## Finding your Plex token

The easiest way:

1. Open Plex Web → click any media item → `···` → Get Info → View XML.
2. The URL contains `X-Plex-Token=<your-token>`.

---

## Google OAuth setup

1. Go to [Google Cloud Console](https://console.cloud.google.com/) → APIs & Services → Credentials.
2. Create an OAuth 2.0 Client ID → choose **TV and Limited Input devices**.
3. Copy the Client ID and Client Secret into `config.toml`.
4. No redirect URI is needed — yt-plex uses the device flow.

---

## Volume reference

| Mount | Purpose | Type |
|-------|---------|------|
| `/config` | `config.toml` | bind mount (`./config`) |
| `/data` | SQLite DB + thumbnail cache | named volume (`yt-plex-data`) |
| `/media` | Plex media output directory | bind mount (your path) |

---

## Useful commands

```bash
# View logs
docker compose logs -f yt-plex

# Open a shell inside the container
docker compose exec yt-plex bash

# Force a full yt-dlp update inside the running container
docker compose exec yt-plex bash -c "curl -fsSL https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp -o /usr/local/bin/yt-dlp"

# Stop
docker compose down

# Stop and wipe all persistent data (destructive!)
docker compose down -v
```
