# Google OAuth Authentication

**Date:** 2026-04-05  
**Status:** Approved

## Summary

Replace password-based admin authentication with Google OAuth. Admin emails are configured in `config.toml`; only users whose Google account email matches the list are granted access. Sessions are stored in the existing SQLite `sessions` table.

## Config Changes

`AuthConfig` loses `admin_password_hash` and gains `admin_emails`. A new `[google_oauth]` section is added:

```toml
[auth]
admin_emails = ["you@gmail.com"]

[google_oauth]
client_id = "..."
client_secret = "..."
redirect_uri = "http://yourserver:3000/api/auth/callback"
```

The `hash-password` CLI subcommand is removed.

## Auth Flow

### Routes

- **`GET /api/auth/login`** — Builds the Google authorization URL requesting only the `email` scope. Generates a random state string, stores it in a short-lived in-memory `HashMap<String, Instant>` on `AppState`, then issues a `302` redirect to Google.

- **`GET /api/auth/callback?code=...&state=...`** — OAuth callback handler:
  1. Validate the `state` parameter against the in-memory map (reject if missing or expired). Expired entries are pruned on each callback request.
  2. POST to `https://oauth2.googleapis.com/token` with the code and client credentials to obtain an access token.
  3. GET `https://www.googleapis.com/oauth2/v2/userinfo` with the access token to retrieve the user's email.
  4. Check email against `admin_emails`. If not found, return a simple 403 error page.
  5. Generate a session token, insert into the `sessions` table, set an HttpOnly `session=<token>` cookie (7-day expiry, SameSite=Lax), redirect to `/`.

- **`POST /api/logout`** — Unchanged: deletes the session row and clears the cookie.

### CSRF State

States are stored in `AppState` as `Arc<Mutex<HashMap<String, Instant>>>`. TTL is 10 minutes. Cleanup runs at the start of each callback request (remove entries older than 10 minutes).

### Implementation Notes

- HTTP calls to Google use the existing `reqwest` client (no new dependencies).
- No new crates required.
- The existing `SessionToken` extractor and `sessions` table are unchanged.

## Removed

- `AuthConfig.admin_password_hash`
- `auth::hash_password()` and `auth::verify_password()`
- `argon2` crate dependency
- `hash-password` CLI subcommand in `main.rs`
- `POST /api/login` route and `LoginRequest` struct

## Frontend

The login page (`/login`) replaces its password form with a single "Sign in with Google" link pointing to `/api/auth/login`. On 403 from the callback, a brief error message is shown ("Your account is not authorised.") with a retry link.

## Out of Scope

- Regular (non-admin) user login — sessions table is preserved for future use.
- Token refresh — access tokens are only used once during the callback to fetch the email; no refresh token is stored.
- Session revocation beyond logout.
