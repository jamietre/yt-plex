# Google OAuth Authentication Implementation Plan (Device Flow)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace password-based admin login with Google OAuth device flow (RFC 8628), allowing the app to work on a local network without a public redirect URI.

**Architecture:** User clicks "Sign in with Google" → server requests a device code from Google → frontend shows the user_code and polls the server → server polls Google's token endpoint → on success, fetches email, checks admin list, creates SQLite session.

**Tech Stack:** Rust/Axum, reqwest (already present), urlencoding (already added), SvelteKit frontend.

---

## Completed Tasks (do not re-implement)

- **Task 1** (`74ba32f`): Config structs — `AuthConfig.admin_emails`, `GoogleOAuthConfig` (still has `redirect_uri` field — Task A below removes it)
- **Task 2** (`b87094e`): `auth.rs` slimmed to `generate_token` only; `argon2` removed; `urlencoding` added
- **Task 3** (`dadcea3`): `AppState.oauth_states` added; router updated (still points at standard-flow handlers — Tasks B/D below fix this)
- **Task 4** (`7d49641`): Standard OAuth routes implemented — to be replaced by device flow routes in Task D

---

## File Map (Remaining Work)

| File | Change |
|------|--------|
| `crates/common/src/config.rs` | Remove `redirect_uri` from `GoogleOAuthConfig` |
| `crates/server/src/lib.rs` | Change `oauth_states` value type to `DeviceCodeEntry`; update router path |
| `crates/server/src/routes/auth.rs` | Replace standard OAuth handlers with device flow: `device_login`, `device_poll` |
| `crates/server/src/main.rs` | Remove `HashPassword` subcommand |
| `web/src/routes/login/+page.svelte` | Device flow UI: show user_code, poll server |
| `web/src/lib/api.ts` | Add `startDeviceLogin()`, `pollDeviceAuth()`, remove `login()` |
| `README.md` | Update setup (no redirect_uri, device client type, updated config template) |

---

### Task A: Remove redirect_uri from config

**Files:**
- Modify: `crates/common/src/config.rs`

- [ ] **Step 1: Update the test — remove redirect_uri from the TOML fixture**

In `crates/common/src/config.rs`, update the `parses_valid_config` test. Change the `[google_oauth]` section from:
```toml
[google_oauth]
client_id = "fake_client_id"
client_secret = "fake_secret"
redirect_uri = "http://localhost:3000/api/auth/callback"
```
to:
```toml
[google_oauth]
client_id = "fake_client_id"
client_secret = "fake_secret"
```
Remove the `redirect_uri` assertion from the test body.

- [ ] **Step 2: Run test to confirm it fails**

```bash
cargo test -p yt-plex-common parses_valid_config 2>&1 | tail -10
```

Expected: compile error — `GoogleOAuthConfig` still has `redirect_uri`.

- [ ] **Step 3: Remove redirect_uri from GoogleOAuthConfig**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleOAuthConfig {
    pub client_id: String,
    pub client_secret: String,
}
```

- [ ] **Step 4: Run test to confirm it passes**

```bash
cargo test -p yt-plex-common 2>&1 | tail -10
```

Expected: all 3 common tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/common/src/config.rs
git commit -m "feat: remove redirect_uri from GoogleOAuthConfig (device flow)"
```

---

### Task B: Update AppState and router for device flow

**Files:**
- Modify: `crates/server/src/lib.rs`

- [ ] **Step 1: Replace lib.rs**

Write the entire file:

```rust
pub mod auth;
pub mod db;
pub mod plex;
pub mod routes;
pub mod template;
pub mod worker;
pub mod ws;

use anyhow::Result;
use axum::{
    routing::{get, post, put},
    Router,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tower_http::trace::TraceLayer;
use yt_plex_common::config::Config;

use crate::{db::Db, ws::WsHub};

pub struct DeviceCodeEntry {
    pub google_device_code: String,
    pub expires_at: Instant,
    pub interval: u64,
}

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Db>,
    pub config: Arc<std::sync::RwLock<Config>>,
    pub config_path: String,
    pub ws_hub: WsHub,
    pub oauth_states: Arc<Mutex<HashMap<String, DeviceCodeEntry>>>,
    pub http_client: reqwest::Client,
}

pub async fn create_app_state(config: Config, config_path: String) -> Result<AppState> {
    let db_path = {
        let base = std::env::var("XDG_DATA_HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
                std::path::PathBuf::from(home)
                    .join(".local")
                    .join("share")
            });
        base.join("yt-plex").join("db.sqlite")
    };
    if let Some(p) = db_path.parent() {
        std::fs::create_dir_all(p)?;
    }
    let db = Arc::new(Db::open(db_path.to_str().unwrap())?);
    let ws_hub = WsHub::new();
    Ok(AppState {
        db,
        config: Arc::new(std::sync::RwLock::new(config)),
        config_path,
        ws_hub,
        oauth_states: Arc::new(Mutex::new(HashMap::new())),
        http_client: reqwest::Client::new(),
    })
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/api/auth/login", get(routes::auth::device_login))
        .route("/api/auth/poll", get(routes::auth::device_poll))
        .route("/api/logout", post(routes::auth::logout))
        .route("/api/jobs", post(routes::jobs::submit_job))
        .route("/api/jobs", get(routes::jobs::list_jobs))
        .route("/api/settings", get(routes::settings::get_settings))
        .route("/api/settings", put(routes::settings::update_settings))
        .route("/ws", get(ws::ws_handler))
        .fallback(routes::assets::serve_asset)
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}
```

- [ ] **Step 2: Check compilation (expect errors about device_login/device_poll and hash_password)**

```bash
cargo build -p yt-plex-server 2>&1 | grep "^error" | head -10
```

Expected errors (all fixed in subsequent tasks):
- `device_login`/`device_poll` not found in `routes::auth`
- `hash_password` in `main.rs`

- [ ] **Step 3: Commit**

```bash
git add crates/server/src/lib.rs
git commit -m "feat: update AppState for device flow (DeviceCodeEntry, router paths)"
```

---

### Task C: Remove hash-password CLI subcommand

**Files:**
- Modify: `crates/server/src/main.rs`

- [ ] **Step 1: Replace main.rs**

```rust
use anyhow::{Context, Result};
use clap::Parser;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use yt_plex_common::config::{default_config_path, load_config};
use yt_plex_server::{build_router, create_app_state, worker};

#[derive(Parser)]
#[command(name = "yt-plex", about = "YouTube → Plex download server")]
struct Args {
    #[arg(long, env = "YT_PLEX_CONFIG")]
    config: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let config_path = args.config.unwrap_or_else(default_config_path);

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "warn,yt_plex_server=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = load_config(&config_path)
        .with_context(|| format!("loading config from {config_path}"))?;
    let bind = config.server.bind.clone();

    let state = create_app_state(config, config_path).await?;

    {
        let db = Arc::clone(&state.db);
        let config = Arc::clone(&state.config);
        let hub = state.ws_hub.clone();
        tokio::spawn(async move {
            worker::run(db, config, hub).await;
        });
    }

    let app = build_router(state);
    let listener = tokio::net::TcpListener::bind(&bind)
        .await
        .with_context(|| format!("binding to {bind}"))?;

    tracing::info!("listening on {bind}");
    axum::serve(listener, app).await.context("server error")?;

    Ok(())
}
```

- [ ] **Step 2: Build**

```bash
cargo build -p yt-plex-server 2>&1 | grep "^error" | head -10
```

Expected: errors only about `device_login`/`device_poll` not found — `main.rs` errors should be gone.

- [ ] **Step 3: Commit**

```bash
git add crates/server/src/main.rs
git commit -m "chore: remove hash-password CLI subcommand"
```

---

### Task D: Implement device flow routes

**Files:**
- Modify: `crates/server/src/routes/auth.rs`

- [ ] **Step 1: Write the entire file**

```rust
use axum::{
    extract::{FromRequestParts, Query, State},
    http::{header, request::Parts, StatusCode},
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tracing::warn;

use crate::{auth as auth_util, AppState, DeviceCodeEntry};

/// Extracts the raw session token string from the Cookie header (or None).
pub struct SessionToken(pub Option<String>);

impl<S> FromRequestParts<S> for SessionToken
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get(header::COOKIE)
            .and_then(|v| v.to_str().ok())
            .and_then(|cookies| {
                cookies
                    .split(';')
                    .map(|c| c.trim())
                    .find(|c| c.starts_with("session="))
                    .map(|c| c["session=".len()..].to_string())
            });
        Ok(SessionToken(token))
    }
}

// ── Device code initiation ────────────────────────────────────────────────────

#[derive(Deserialize)]
struct GoogleDeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_url: String,
    expires_in: u64,
    interval: u64,
}

#[derive(Serialize)]
pub struct DeviceLoginResponse {
    pub poll_token: String,
    pub user_code: String,
    pub verification_url: String,
    pub expires_in: u64,
    pub interval: u64,
}

pub async fn device_login(State(state): State<AppState>) -> axum::response::Response {
    let client_id = state.config.read().unwrap().google_oauth.client_id.clone();

    let resp = match state
        .http_client
        .post("https://oauth2.googleapis.com/device/code")
        .form(&[("client_id", client_id.as_str()), ("scope", "email")])
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            warn!("device code request failed: {e}");
            return (StatusCode::BAD_GATEWAY, "Failed to contact Google.").into_response();
        }
    };

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        warn!("device code request returned HTTP {status}: {body}");
        return (StatusCode::BAD_GATEWAY, "Failed to start sign-in.").into_response();
    }

    let google_resp: GoogleDeviceCodeResponse = match resp.json().await {
        Ok(d) => d,
        Err(e) => {
            warn!("device code parse failed: {e}");
            return (StatusCode::BAD_GATEWAY, "Unexpected response from Google.").into_response();
        }
    };

    let poll_token = auth_util::generate_token();
    {
        let mut states = state.oauth_states.lock().unwrap();
        states.insert(
            poll_token.clone(),
            DeviceCodeEntry {
                google_device_code: google_resp.device_code,
                expires_at: Instant::now() + Duration::from_secs(google_resp.expires_in),
                interval: google_resp.interval,
            },
        );
    }

    Json(DeviceLoginResponse {
        poll_token,
        user_code: google_resp.user_code,
        verification_url: google_resp.verification_url,
        expires_in: google_resp.expires_in,
        interval: google_resp.interval,
    })
    .into_response()
}

// ── Device code polling ───────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct PollParams {
    pub token: String,
}

#[derive(Serialize)]
pub struct PollResponse {
    pub status: &'static str, // "pending" | "done" | "denied" | "expired" | "error"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

// Google token endpoint response (device flow)
#[derive(Deserialize)]
struct GoogleTokenResponse {
    access_token: Option<String>,
    error: Option<String>,
    interval: Option<u64>,
}

#[derive(Deserialize)]
struct UserInfo {
    email: String,
}

pub async fn device_poll(
    State(state): State<AppState>,
    Query(params): Query<PollParams>,
) -> axum::response::Response {
    // Prune expired entries
    {
        let mut states = state.oauth_states.lock().unwrap();
        states.retain(|_, e| e.expires_at > Instant::now());
    }

    let (google_device_code, client_id, client_secret) = {
        let states = state.oauth_states.lock().unwrap();
        match states.get(&params.token) {
            None => {
                return Json(PollResponse {
                    status: "expired",
                    interval: None,
                    message: None,
                })
                .into_response();
            }
            Some(entry) => {
                let cfg = state.config.read().unwrap();
                (
                    entry.google_device_code.clone(),
                    cfg.google_oauth.client_id.clone(),
                    cfg.google_oauth.client_secret.clone(),
                )
            }
        }
    };

    // Poll Google's token endpoint
    let token_resp = match state
        .http_client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("device_code", google_device_code.as_str()),
            (
                "grant_type",
                "urn:ietf:params:oauth:grant-type:device_code",
            ),
        ])
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            warn!("token poll request failed: {e}");
            return Json(PollResponse {
                status: "error",
                interval: None,
                message: Some("Failed to contact Google.".into()),
            })
            .into_response();
        }
    };

    let google_token: GoogleTokenResponse = match token_resp.json().await {
        Ok(d) => d,
        Err(e) => {
            warn!("token poll parse failed: {e}");
            return Json(PollResponse {
                status: "error",
                interval: None,
                message: Some("Unexpected response from Google.".into()),
            })
            .into_response();
        }
    };

    match google_token.error.as_deref() {
        Some("authorization_pending") => {
            return Json(PollResponse {
                status: "pending",
                interval: None,
                message: None,
            })
            .into_response();
        }
        Some("slow_down") => {
            return Json(PollResponse {
                status: "pending",
                interval: google_token.interval,
                message: None,
            })
            .into_response();
        }
        Some("access_denied") => {
            state.oauth_states.lock().unwrap().remove(&params.token);
            return Json(PollResponse {
                status: "denied",
                interval: None,
                message: Some("Access was denied.".into()),
            })
            .into_response();
        }
        Some(other) => {
            warn!("Google token error: {other}");
            state.oauth_states.lock().unwrap().remove(&params.token);
            return Json(PollResponse {
                status: "error",
                interval: None,
                message: Some(format!("Google error: {other}")),
            })
            .into_response();
        }
        None => {} // fall through to access_token handling
    }

    let access_token = match google_token.access_token {
        Some(t) => t,
        None => {
            warn!("token poll returned neither access_token nor error");
            return Json(PollResponse {
                status: "error",
                interval: None,
                message: Some("Unexpected response from Google.".into()),
            })
            .into_response();
        }
    };

    // Fetch user's email
    let userinfo_resp = match state
        .http_client
        .get("https://www.googleapis.com/oauth2/v2/userinfo")
        .bearer_auth(&access_token)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            warn!("userinfo request failed: {e}");
            return Json(PollResponse {
                status: "error",
                interval: None,
                message: Some("Failed to fetch user info from Google.".into()),
            })
            .into_response();
        }
    };

    if !userinfo_resp.status().is_success() {
        let status = userinfo_resp.status();
        let body = userinfo_resp.text().await.unwrap_or_default();
        warn!("userinfo returned HTTP {status}: {body}");
        return Json(PollResponse {
            status: "error",
            interval: None,
            message: Some("Failed to fetch user info.".into()),
        })
        .into_response();
    }

    let userinfo: UserInfo = match userinfo_resp.json().await {
        Ok(d) => d,
        Err(e) => {
            warn!("userinfo parse failed: {e}");
            return Json(PollResponse {
                status: "error",
                interval: None,
                message: Some("Unexpected user info response.".into()),
            })
            .into_response();
        }
    };

    // Check against admin list
    let is_admin = {
        let cfg = state.config.read().unwrap();
        cfg.auth
            .admin_emails
            .iter()
            .any(|e| e.eq_ignore_ascii_case(&userinfo.email))
    };

    if !is_admin {
        warn!("device flow login rejected for {}", userinfo.email);
        state.oauth_states.lock().unwrap().remove(&params.token);
        return Json(PollResponse {
            status: "denied",
            interval: None,
            message: Some("Your account is not authorised.".into()),
        })
        .into_response();
    }

    // Create session
    let session_token = auth_util::generate_token();
    if let Err(e) = state.db.insert_session(&session_token) {
        warn!("insert session error: {e}");
        return Json(PollResponse {
            status: "error",
            interval: None,
            message: Some("Server error creating session.".into()),
        })
        .into_response();
    }

    // Clean up state
    state.oauth_states.lock().unwrap().remove(&params.token);

    let cookie = format!(
        "session={}; Path=/; HttpOnly; SameSite=Lax; Max-Age=604800",
        session_token
    );
    (
        StatusCode::OK,
        [(header::SET_COOKIE, cookie)],
        Json(PollResponse {
            status: "done",
            interval: None,
            message: None,
        }),
    )
        .into_response()
}

// ── Logout ────────────────────────────────────────────────────────────────────

pub async fn logout(
    State(state): State<AppState>,
    SessionToken(token): SessionToken,
) -> impl IntoResponse {
    if let Some(t) = token {
        let _ = state.db.delete_session(&t);
    }
    let clear = "session=; Path=/; HttpOnly; Max-Age=0";
    (StatusCode::OK, [(header::SET_COOKIE, clear)], "OK")
}
```

- [ ] **Step 2: Build and fix any compile errors**

```bash
cargo build -p yt-plex-server 2>&1 | grep "^error" | head -20
```

Expected: clean build (0 errors).

- [ ] **Step 3: Run all tests**

```bash
cargo test -p yt-plex-server 2>&1 | tail -20
```

Expected: all existing tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/server/src/routes/auth.rs
git commit -m "feat: implement Google device flow auth (RFC 8628)"
```

---

### Task E: Update frontend login page

**Files:**
- Modify: `web/src/routes/login/+page.svelte`
- Modify: `web/src/lib/api.ts`

- [ ] **Step 1: Add API functions to api.ts**

Read `web/src/lib/api.ts` first. Add these two functions (remove the old `login()` function if present):

```typescript
export interface DeviceLoginResponse {
  poll_token: string;
  user_code: string;
  verification_url: string;
  expires_in: number;
  interval: number;
}

export interface PollResponse {
  status: 'pending' | 'done' | 'denied' | 'expired' | 'error';
  interval?: number;
  message?: string;
}

export async function startDeviceLogin(): Promise<DeviceLoginResponse> {
  const res = await fetch('/api/auth/login');
  if (!res.ok) throw new Error(`${res.status}`);
  return res.json();
}

export async function pollDeviceAuth(token: string): Promise<PollResponse> {
  const res = await fetch(`/api/auth/poll?token=${encodeURIComponent(token)}`);
  if (!res.ok) throw new Error(`${res.status}`);
  return res.json();
}
```

- [ ] **Step 2: Replace the login page**

Write the entire file `web/src/routes/login/+page.svelte`:

```svelte
<script lang="ts">
    import { startDeviceLogin, pollDeviceAuth, type DeviceLoginResponse } from '$lib/api';

    type Phase = 'idle' | 'waiting' | 'done' | 'error';

    let phase = $state<Phase>('idle');
    let deviceInfo = $state<DeviceLoginResponse | null>(null);
    let errorMsg = $state('');
    let pollTimer: ReturnType<typeof setTimeout> | null = null;

    async function startLogin() {
        phase = 'idle';
        errorMsg = '';
        deviceInfo = null;
        try {
            deviceInfo = await startDeviceLogin();
            phase = 'waiting';
            schedulePoll(deviceInfo.interval);
        } catch {
            phase = 'error';
            errorMsg = 'Failed to start sign-in. Please try again.';
        }
    }

    function schedulePoll(intervalSecs: number) {
        pollTimer = setTimeout(doPoll, intervalSecs * 1000);
    }

    async function doPoll() {
        if (!deviceInfo) return;
        try {
            const result = await pollDeviceAuth(deviceInfo.poll_token);
            if (result.status === 'pending') {
                schedulePoll(result.interval ?? deviceInfo.interval);
            } else if (result.status === 'done') {
                phase = 'done';
                window.location.href = '/';
            } else if (result.status === 'expired') {
                phase = 'error';
                errorMsg = 'Sign-in timed out. Please try again.';
            } else {
                phase = 'error';
                errorMsg = result.message ?? 'Sign-in failed. Please try again.';
            }
        } catch {
            schedulePoll(deviceInfo.interval);
        }
    }
</script>

<main>
    <h2>Sign in</h2>

    {#if phase === 'idle' || phase === 'error'}
        {#if errorMsg}<p class="error">{errorMsg}</p>{/if}
        <button onclick={startLogin}>Sign in with Google</button>

    {:else if phase === 'waiting' && deviceInfo}
        <p>Open the following URL and enter the code below:</p>
        <p><a href={deviceInfo.verification_url} target="_blank" rel="noreferrer">{deviceInfo.verification_url}</a></p>
        <p class="code">{deviceInfo.user_code}</p>
        <p class="hint">Waiting for authorisation…</p>

    {:else if phase === 'done'}
        <p>Signed in! Redirecting…</p>
    {/if}
</main>

<style>
    main {
        max-width: 400px;
        margin: 6rem auto;
        padding: 2rem;
        font-family: sans-serif;
        text-align: center;
        border: 1px solid #333;
        border-radius: 8px;
        background: #1a1a1a;
        color: #fff;
    }
    h2 { margin-bottom: 1.5rem; }
    button {
        padding: 0.6rem 1.4rem;
        background: #fff;
        color: #333;
        border: none;
        border-radius: 4px;
        cursor: pointer;
        font-weight: 500;
        font-size: 1rem;
    }
    button:hover { background: #e8e8e8; }
    .code {
        font-size: 2rem;
        font-weight: bold;
        letter-spacing: 0.2em;
        margin: 1rem 0;
        font-family: monospace;
    }
    .hint { color: #888; font-size: 0.9rem; }
    .error { color: #f66; }
    a { color: #4af; }
</style>
```

- [ ] **Step 3: Build the frontend**

```bash
cd web && pnpm build 2>&1 | tail -10
```

Expected: `✔ done`

- [ ] **Step 4: Commit**

```bash
git add web/src/routes/login/+page.svelte web/src/lib/api.ts web/build/
git commit -m "feat: device flow login UI — show user code, poll for completion"
```

---

### Task F: Update README

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update the config template and instructions**

In `README.md`:

1. Update the `[auth]` + `[google_oauth]` config example to:
   ```toml
   [auth]
   admin_emails = ["you@gmail.com"]

   [google_oauth]
   client_id = "your-client-id.apps.googleusercontent.com"
   client_secret = "your-client-secret"
   ```

2. Replace any instructions about `hash-password`, `redirect_uri`, or "Web application" OAuth with:
   > **Creating Google OAuth credentials:** Go to [console.cloud.google.com](https://console.cloud.google.com) → APIs & Services → Credentials → Create OAuth 2.0 Client ID. Choose type **"TVs and Limited Input devices"**. No redirect URIs are needed.

3. Remove any remaining reference to the `hash-password` command.

4. Update the Usage section to describe the device flow login:
   > Log in by clicking "Sign in with Google", then visit the displayed URL on any device and enter the shown code.

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "docs: update setup for device flow OAuth"
```

---

### Task G: Final verification

- [ ] **Step 1: Run the full test suite**

```bash
cargo test 2>&1 | tail -20
```

Expected: all tests pass (≥15 passing, 0 failing).

- [ ] **Step 2: Release build**

```bash
cargo build --release -p yt-plex-server 2>&1 | grep "^error" | head -10
```

Expected: clean build.

- [ ] **Step 3: Frontend build**

```bash
cd web && pnpm build 2>&1 | tail -5
```

Expected: `✔ done`

- [ ] **Step 4: Push**

```bash
git push -u origin feat/google-oauth
```
