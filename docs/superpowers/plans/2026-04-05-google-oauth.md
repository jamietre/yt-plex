# Google OAuth Authentication Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace password-based admin login with Google OAuth, gating access by a configurable list of admin email addresses.

**Architecture:** The browser is redirected to Google's authorization endpoint; after consent, Google redirects back to `/api/auth/callback` where the server exchanges the code for an access token, fetches the user's email from Google's userinfo endpoint, verifies it against `admin_emails`, and creates a SQLite-backed session as before.

**Tech Stack:** Rust/Axum, reqwest (already present), urlencoding (new, tiny dep), SvelteKit frontend.

---

## File Map

| File | Change |
|------|--------|
| `crates/common/src/config.rs` | Replace `AuthConfig.admin_password_hash` with `admin_emails: Vec<String>`; add `GoogleOAuthConfig` struct |
| `crates/server/Cargo.toml` | Remove `argon2`; add `urlencoding = "2"` |
| `crates/server/src/auth.rs` | Remove `hash_password`, `verify_password`; keep `generate_token` |
| `crates/server/src/lib.rs` | Add `oauth_states` field to `AppState`; update router |
| `crates/server/src/routes/auth.rs` | Remove `login`/`LoginRequest`; add `oauth_login`, `oauth_callback`, `error_page` |
| `crates/server/src/main.rs` | Remove `HashPassword` subcommand |
| `web/src/routes/login/+page.svelte` | Replace password form with "Sign in with Google" link |
| `web/src/lib/api.ts` | Remove `login()` function |
| `README.md` | Update setup section (no more `hash-password`) |

---

### Task 1: Update config structs

**Files:**
- Modify: `crates/common/src/config.rs`

- [ ] **Step 1: Update the config test to reflect the new shape**

Replace the existing `parses_valid_config` test in `crates/common/src/config.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_config() {
        let toml = r#"
[server]
bind = "127.0.0.1:3000"

[auth]
admin_emails = ["admin@example.com"]

[google_oauth]
client_id = "fake_client_id"
client_secret = "fake_secret"
redirect_uri = "http://localhost:3000/api/auth/callback"

[plex]
url = "http://localhost:32400"
token = "mytoken"
library_section_id = "1"

[output]
base_path = "/mnt/plex"
path_template = "{channel}/{date} - {title}.{ext}"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.server.bind, "127.0.0.1:3000");
        assert_eq!(config.auth.admin_emails, vec!["admin@example.com"]);
        assert_eq!(config.google_oauth.client_id, "fake_client_id");
        assert_eq!(config.plex.library_section_id, "1");
    }
}
```

- [ ] **Step 2: Run the test to confirm it fails**

```bash
cargo test -p yt-plex-common parses_valid_config 2>&1 | tail -20
```

Expected: compile error — `AuthConfig` still has `admin_password_hash`, `Config` has no `google_oauth`.

- [ ] **Step 3: Replace the structs**

Replace the `AuthConfig` struct and add `GoogleOAuthConfig` in `crates/common/src/config.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub admin_emails: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleOAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub auth: AuthConfig,
    pub google_oauth: GoogleOAuthConfig,
    pub plex: PlexConfig,
    pub output: OutputConfig,
}
```

- [ ] **Step 4: Run the test to confirm it passes**

```bash
cargo test -p yt-plex-common parses_valid_config 2>&1 | tail -10
```

Expected: `test tests::parses_valid_config ... ok`

- [ ] **Step 5: Commit**

```bash
git add crates/common/src/config.rs
git commit -m "feat: replace admin_password_hash with admin_emails and GoogleOAuthConfig"
```

---

### Task 2: Slim auth.rs and remove argon2

**Files:**
- Modify: `crates/server/src/auth.rs`
- Modify: `crates/server/Cargo.toml`

- [ ] **Step 1: Verify the generate_token test still passes before changes**

```bash
cargo test -p yt-plex-server generate_token 2>&1 | tail -10
```

Expected: `test auth::tests::generate_token_is_64_hex_chars ... ok`

- [ ] **Step 2: Replace auth.rs, removing hash/verify and their tests**

Write the entire file:

```rust
use rand::Rng;

pub fn generate_token() -> String {
    let bytes: [u8; 32] = rand::thread_rng().gen();
    hex::encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_token_is_64_hex_chars() {
        let tok = generate_token();
        assert_eq!(tok.len(), 64);
        assert!(tok.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
```

- [ ] **Step 3: Remove argon2, add urlencoding in crates/server/Cargo.toml**

Remove this line:
```
argon2         = "0.5"
```

Add this line after `hex`:
```
urlencoding    = "2"
```

- [ ] **Step 4: Confirm generate_token test still passes**

```bash
cargo test -p yt-plex-server generate_token 2>&1 | tail -10
```

Expected: `test auth::tests::generate_token_is_64_hex_chars ... ok`

- [ ] **Step 5: Commit**

```bash
git add crates/server/src/auth.rs crates/server/Cargo.toml
git commit -m "chore: remove argon2 password auth, add urlencoding dep"
```

---

### Task 3: Add oauth_states to AppState and update router

**Files:**
- Modify: `crates/server/src/lib.rs`

- [ ] **Step 1: Replace lib.rs with the updated AppState and router**

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

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Db>,
    pub config: Arc<std::sync::RwLock<Config>>,
    pub config_path: String,
    pub ws_hub: WsHub,
    pub oauth_states: Arc<Mutex<HashMap<String, Instant>>>,
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
    })
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/api/auth/login", get(routes::auth::oauth_login))
        .route("/api/auth/callback", get(routes::auth::oauth_callback))
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

- [ ] **Step 2: Check it compiles (routes/auth.rs still has old code — expect a compile error there)**

```bash
cargo build -p yt-plex-server 2>&1 | grep "^error" | head -20
```

Expected: errors about `oauth_login`/`oauth_callback` not found — that's fine, we implement them next.

- [ ] **Step 3: Commit**

```bash
git add crates/server/src/lib.rs
git commit -m "feat: add oauth_states to AppState, update router to OAuth routes"
```

---

### Task 4: Implement OAuth routes

**Files:**
- Modify: `crates/server/src/routes/auth.rs`

- [ ] **Step 1: Replace routes/auth.rs entirely**

```rust
use axum::{
    body::Body,
    extract::{FromRequestParts, Query, State},
    http::{header, request::Parts, Response, StatusCode},
    response::{IntoResponse, Redirect},
};
use serde::Deserialize;
use std::time::{Duration, Instant};
use tracing::warn;

use crate::{auth as auth_util, AppState};

const STATE_TTL: Duration = Duration::from_secs(600);

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

pub async fn oauth_login(State(state): State<AppState>) -> impl IntoResponse {
    let oauth_state = auth_util::generate_token();
    {
        let mut states = state.oauth_states.lock().unwrap();
        states.insert(oauth_state.clone(), Instant::now());
    }
    let cfg = state.config.read().unwrap();
    let url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth\
         ?response_type=code\
         &client_id={}\
         &redirect_uri={}\
         &scope=email\
         &state={}",
        urlencoding::encode(&cfg.google_oauth.client_id),
        urlencoding::encode(&cfg.google_oauth.redirect_uri),
        oauth_state, // hex string, no encoding needed
    );
    Redirect::to(&url)
}

#[derive(Deserialize)]
pub struct CallbackParams {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct UserInfo {
    email: String,
}

pub async fn oauth_callback(
    State(state): State<AppState>,
    Query(params): Query<CallbackParams>,
) -> axum::response::Response {
    // Prune expired states on every callback
    {
        let mut states = state.oauth_states.lock().unwrap();
        states.retain(|_, t| t.elapsed() < STATE_TTL);
    }

    if let Some(err) = params.error {
        warn!("OAuth error from Google: {err}");
        return error_page("Google sign-in was cancelled or failed.");
    }

    let code = match params.code {
        Some(c) => c,
        None => return error_page("Missing code parameter."),
    };
    let incoming_state = match params.state {
        Some(s) => s,
        None => return error_page("Missing state parameter."),
    };

    // Validate and consume state (prevents CSRF)
    let valid = {
        let mut states = state.oauth_states.lock().unwrap();
        states
            .remove(&incoming_state)
            .map(|t| t.elapsed() < STATE_TTL)
            .unwrap_or(false)
    };
    if !valid {
        return error_page("Invalid or expired state. Please try again.");
    }

    let (client_id, client_secret, redirect_uri) = {
        let cfg = state.config.read().unwrap();
        (
            cfg.google_oauth.client_id.clone(),
            cfg.google_oauth.client_secret.clone(),
            cfg.google_oauth.redirect_uri.clone(),
        )
    };

    // Exchange code for access token
    let http = reqwest::Client::new();
    let token_resp = match http
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
        ])
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            warn!("token exchange request failed: {e}");
            return error_page("Failed to contact Google. Please try again.");
        }
    };

    let token_data: TokenResponse = match token_resp.json().await {
        Ok(d) => d,
        Err(e) => {
            warn!("token exchange parse failed: {e}");
            return error_page("Unexpected response from Google. Please try again.");
        }
    };

    // Fetch user's email from userinfo endpoint
    let userinfo_resp = match http
        .get("https://www.googleapis.com/oauth2/v2/userinfo")
        .bearer_auth(&token_data.access_token)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            warn!("userinfo request failed: {e}");
            return error_page("Failed to fetch user info from Google.");
        }
    };

    let userinfo: UserInfo = match userinfo_resp.json().await {
        Ok(d) => d,
        Err(e) => {
            warn!("userinfo parse failed: {e}");
            return error_page("Unexpected user info response from Google.");
        }
    };

    // Check email against configured admin list
    let is_admin = {
        let cfg = state.config.read().unwrap();
        cfg.auth
            .admin_emails
            .iter()
            .any(|e| e.eq_ignore_ascii_case(&userinfo.email))
    };

    if !is_admin {
        warn!("OAuth login rejected for {}", userinfo.email);
        return (
            StatusCode::FORBIDDEN,
            axum::response::Html(
                "<h1>Access denied</h1>\
                 <p>Your account is not authorised to access this application.</p>\
                 <p><a href=\"/api/auth/login\">Try a different account</a></p>",
            ),
        )
            .into_response();
    }

    // Create session and set cookie
    let token = auth_util::generate_token();
    if let Err(e) = state.db.insert_session(&token) {
        warn!("insert session error: {e}");
        return error_page("Server error creating session.");
    }

    let cookie = format!(
        "session={}; Path=/; HttpOnly; SameSite=Lax; Max-Age=604800",
        token
    );
    Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header(header::LOCATION, "/")
        .header(header::SET_COOKIE, cookie)
        .body(Body::empty())
        .unwrap()
}

pub async fn logout(State(state): State<AppState>, SessionToken(token): SessionToken) -> impl IntoResponse {
    if let Some(t) = token {
        let _ = state.db.delete_session(&t);
    }
    let clear = "session=; Path=/; HttpOnly; Max-Age=0";
    (StatusCode::OK, [(header::SET_COOKIE, clear)], "OK")
}

fn error_page(msg: &str) -> axum::response::Response {
    let html = format!(
        "<h1>Sign-in failed</h1>\
         <p>{msg}</p>\
         <p><a href=\"/api/auth/login\">Try again</a></p>"
    );
    (StatusCode::BAD_REQUEST, axum::response::Html(html)).into_response()
}
```

- [ ] **Step 2: Build and fix any compile errors**

```bash
cargo build -p yt-plex-server 2>&1 | grep "^error" | head -30
```

Expected: clean build (0 errors). If there are errors, fix them before proceeding.

- [ ] **Step 3: Run all tests**

```bash
cargo test -p yt-plex-server 2>&1 | tail -20
```

Expected: all existing db tests still pass.

- [ ] **Step 4: Commit**

```bash
git add crates/server/src/routes/auth.rs
git commit -m "feat: implement Google OAuth login and callback routes"
```

---

### Task 5: Remove hash-password CLI subcommand

**Files:**
- Modify: `crates/server/src/main.rs`

- [ ] **Step 1: Replace main.rs, removing the HashPassword subcommand**

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

- [ ] **Step 2: Build and run all tests**

```bash
cargo build -p yt-plex-server 2>&1 | grep "^error" | head -10
cargo test 2>&1 | tail -20
```

Expected: clean build, all tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/server/src/main.rs
git commit -m "chore: remove hash-password CLI subcommand"
```

---

### Task 6: Update frontend login page

**Files:**
- Modify: `web/src/routes/login/+page.svelte`
- Modify: `web/src/lib/api.ts`

- [ ] **Step 1: Replace login page with a "Sign in with Google" link**

Write the entire file at `web/src/routes/login/+page.svelte`:

```svelte
<main>
    <h2>Sign in</h2>
    <a href="/api/auth/login" class="google-btn">Sign in with Google</a>
</main>

<style>
    main {
        max-width: 360px;
        margin: 6rem auto;
        padding: 2rem;
        font-family: sans-serif;
        text-align: center;
        border: 1px solid #333;
        border-radius: 8px;
        background: #1a1a1a;
    }
    h2 { margin-bottom: 1.5rem; color: #fff; }
    .google-btn {
        display: inline-block;
        padding: 0.6rem 1.4rem;
        background: #fff;
        color: #333;
        border-radius: 4px;
        text-decoration: none;
        font-weight: 500;
    }
    .google-btn:hover { background: #e8e8e8; }
</style>
```

- [ ] **Step 2: Remove the login() function from api.ts**

Read `web/src/lib/api.ts` first, then remove the `login()` export and the `LoginRequest`/`LoginResponse` types if present. Keep `logout()`, `listJobs()`, `submitJob()`, `getSettings()`, `updateSettings()`.

The login page no longer calls the API — it navigates directly to `/api/auth/login`.

- [ ] **Step 3: Build the frontend**

```bash
cd web && pnpm build 2>&1 | tail -10
```

Expected: `✔ done` with no errors.

- [ ] **Step 4: Commit**

```bash
git add web/src/routes/login/+page.svelte web/src/lib/api.ts web/build/
git commit -m "feat: replace password login form with Sign in with Google"
```

---

### Task 7: Update README

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update the setup section**

In `README.md`:

1. Remove the "Generate a password hash" step and the `hash-password` command example.
2. Replace the `[auth]` section in the config template with:
   ```toml
   [auth]
   admin_emails = ["you@gmail.com"]

   [google_oauth]
   client_id = "your-client-id.apps.googleusercontent.com"
   client_secret = "your-client-secret"
   redirect_uri = "http://yourserver:3000/api/auth/callback"
   ```
3. Add a note explaining how to create a Google OAuth app:
   > **Creating Google OAuth credentials:** Go to [console.cloud.google.com](https://console.cloud.google.com) → APIs & Services → Credentials → Create OAuth 2.0 Client ID (type: Web application). Add your `redirect_uri` as an Authorised redirect URI.

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "docs: update setup instructions for Google OAuth"
```

---

### Task 8: Final verification

- [ ] **Step 1: Run the full test suite**

```bash
cargo test 2>&1 | tail -20
```

Expected: all tests pass (≥15 passing, 0 failing).

- [ ] **Step 2: Do a release build to catch any lingering issues**

```bash
cargo build --release -p yt-plex-server 2>&1 | grep "^error" | head -10
```

Expected: clean build.

- [ ] **Step 3: Push**

```bash
git push
```
