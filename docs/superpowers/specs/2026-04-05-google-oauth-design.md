# Google OAuth Authentication (Device Flow)

**Date:** 2026-04-05  
**Status:** Approved — revised to use Device Authorization Grant (RFC 8628)

## Summary

Replace password-based admin authentication with Google OAuth using the **Device Authorization Grant (device flow)**. The app runs on a local network and is not internet-exposed, so a redirect URI is not viable. Device flow avoids this by having the user authenticate on Google's servers directly.

Admin emails are configured in `config.toml`; only users whose Google account email matches the list are granted access. Sessions are stored in the existing SQLite `sessions` table.

## Google Cloud Setup

Create OAuth credentials of type **"TVs and Limited Input devices"** (not "Web application") in Google Cloud Console. This unlocks device flow. No authorized redirect URIs are needed.

## Config Changes

`AuthConfig` has `admin_emails: Vec<String>` (already in place). `GoogleOAuthConfig` has `client_id` and `client_secret` only — **no `redirect_uri`**:

```toml
[auth]
admin_emails = ["you@gmail.com"]

[google_oauth]
client_id = "your-client-id.apps.googleusercontent.com"
client_secret = "your-client-secret"
```

The `hash-password` CLI subcommand is removed.

## Auth Flow

### Step 1 — Initiate (`GET /api/auth/login`)

1. POST to `https://oauth2.googleapis.com/device/code` with `client_id` and `scope=email`
2. Google returns `device_code`, `user_code`, `verification_url`, `expires_in`, `interval`
3. Generate a server-side `poll_token` (random hex), store in `oauth_states`:
   ```
   poll_token → DeviceCodeEntry { google_device_code, expires_at, interval }
   ```
4. Return JSON: `{ poll_token, user_code, verification_url, expires_in, interval }`

### Step 2 — Poll (`GET /api/auth/poll?token=<poll_token>`)

Browser calls this every `interval` seconds.

1. Look up `poll_token` in `oauth_states`. If missing or expired → `{ "status": "expired" }`
2. POST to `https://oauth2.googleapis.com/token` with `client_id`, `client_secret`, `device_code`, `grant_type=urn:ietf:params:oauth:grant-type:device_code`
3. If Google responds `authorization_pending` → `{ "status": "pending" }`
4. If Google responds `slow_down` → `{ "status": "pending", "interval": N }`
5. If Google returns access token:
   - GET `https://www.googleapis.com/oauth2/v2/userinfo` with bearer token
   - Check email against `admin_emails`; if not found → `{ "status": "denied" }`
   - Create session in SQLite, set `session=<token>; HttpOnly; SameSite=Lax; Max-Age=604800` cookie
   - Return `{ "status": "done" }` with Set-Cookie header
6. Any other Google error (expired_token, access_denied) → `{ "status": "error", "message": "..." }`

### Step 3 — Logout (`POST /api/logout`)

Unchanged: deletes session row, clears cookie.

## State Storage

`oauth_states: Arc<Mutex<HashMap<String, DeviceCodeEntry>>>` on `AppState`.

```rust
struct DeviceCodeEntry {
    google_device_code: String,
    expires_at: Instant,
    interval: u64,
}
```

Cleanup: prune expired entries at the start of each poll request.

## Frontend (Login Page)

1. "Sign in with Google" button → calls `GET /api/auth/login`
2. Shows instructions: "Open [verification_url] and enter code **[user_code]**"
3. Polls `/api/auth/poll?token=<poll_token>` every `interval` seconds
4. On `done`: session cookie is already set; `window.location.href = "/"`
5. On `expired` or `error`: show message + "Try again" button that resets to step 1

## Removed

- `AuthConfig.admin_password_hash`
- `auth::hash_password()` and `auth::verify_password()`
- `argon2` crate dependency
- `hash-password` CLI subcommand
- `POST /api/login` (password route)
- `GoogleOAuthConfig.redirect_uri`
- Standard OAuth `oauth_callback` route (`/api/auth/callback`)

## Out of Scope

- Regular (non-admin) user login
- Token refresh
- Session revocation beyond logout
