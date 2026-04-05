# User Profiles Implementation Plan

**Goal:** Add user profiles so multiple household members can each maintain their own channel subscriptions and video ignore lists. Non-authenticated users pick a profile on first visit; admins are auto-assigned to their linked profile on login.

**Architecture:** Three new SQLite tables (`profiles`, `profile_channels`, `profile_video_ignores`). Profile selection stored in a `yt_plex_profile` cookie. Admin profiles are created automatically on first OAuth login and hidden from the public picker. Existing `videos.ignored_at` remains as an admin-level global suppress; per-user ignores move to `profile_video_ignores`.

**Tech Stack:** Rust, Axum 0.8, rusqlite, SvelteKit 5 (Svelte runes), TypeScript.

---

## Design Decisions

- **Channel subscriptions: opt-in.** Each profile explicitly subscribes to channels. New channels don't auto-appear for existing profiles.
- **Ignore lists: per-profile.** `videos.ignored_at` stays as admin global suppress. User ignores go in `profile_video_ignores`.
- **Session storage: cookie.** `yt_plex_profile={id}` cookie for all users. Set automatically on admin OAuth login.
- **Admin profiles hidden** from public picker via `is_admin_profile = 1` flag. Auto-created on first login, linked by Google email.

---

## File Map

| File | Change |
|------|--------|
| `crates/common/src/models.rs` | Add `Profile` struct |
| `crates/server/src/db.rs` | Add 3 new tables to SCHEMA; add profile/channel/ignore DB methods |
| `crates/server/src/routes/mod.rs` | Add `pub mod profiles;` |
| `crates/server/src/routes/profiles.rs` | **Create** — profile CRUD + session cookie endpoints |
| `crates/server/src/routes/auth.rs` | Auto-create/link admin profile on OAuth completion; set cookie |
| `crates/server/src/routes/channels.rs` | Filter `list_channels` by profile subscriptions |
| `crates/server/src/routes/videos.rs` | Use `profile_video_ignores` for ignore/unignore |
| `crates/server/src/lib.rs` | Add new routes to router; add profile extractor |
| `web/src/lib/api.ts` | Add `Profile` type and profile API functions |
| `web/src/routes/select-profile/+page.svelte` | **Create** — profile picker page |
| `web/src/routes/+layout.svelte` | Redirect to `/select-profile` if no profile cookie |
| `web/src/routes/admin/+page.svelte` | Add Profiles tab (create/delete, channel subscriptions) |

---

## Database Schema

```sql
CREATE TABLE profiles (
    id               INTEGER PRIMARY KEY,
    name             TEXT    NOT NULL UNIQUE,
    linked_email     TEXT,           -- Google email for admin profiles; NULL otherwise
    is_admin_profile INTEGER NOT NULL DEFAULT 0,
    created_at       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
);

-- Channels a profile has subscribed to (opt-in)
CREATE TABLE profile_channels (
    profile_id  INTEGER NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    channel_id  INTEGER NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    PRIMARY KEY (profile_id, channel_id)
);

-- Per-profile video ignores
CREATE TABLE profile_video_ignores (
    profile_id  INTEGER NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    youtube_id  TEXT    NOT NULL,
    ignored_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    PRIMARY KEY (profile_id, youtube_id)
);
```

---

## API Endpoints

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/api/profiles` | any | List profiles where `is_admin_profile=0` |
| `POST` | `/api/profiles` | admin | Create a non-admin profile |
| `DELETE` | `/api/profiles/{id}` | admin | Delete profile (cascades ignores + subscriptions) |
| `GET` | `/api/profile-session` | any | Return current profile from cookie |
| `POST` | `/api/profile-session` | any | Set `yt_plex_profile` cookie `{profile_id}` |
| `DELETE` | `/api/profile-session` | any | Clear profile cookie |
| `GET` | `/api/profiles/{id}/channels` | profile owner or admin | List channels subscribed by profile |
| `PUT` | `/api/profiles/{id}/channels/{cid}` | profile owner or admin | Subscribe channel to profile |
| `DELETE` | `/api/profiles/{id}/channels/{cid}` | profile owner or admin | Unsubscribe channel from profile |

**Existing endpoints modified:**
- `GET /api/channels` — filter to `profile_channels` for current profile (admin sees all)
- `GET /api/channels/{id}/videos` — filter ignores via `profile_video_ignores`
- `POST /api/videos/{id}/ignore` — insert into `profile_video_ignores`
- `DELETE /api/videos/{id}/ignore` — delete from `profile_video_ignores`

---

## Tasks

### Task 1: Database — new tables and DB methods

- [ ] Add the 3 new tables to the SCHEMA constant in `db.rs`
- [ ] Add `Profile` struct to `crates/common/src/models.rs`
- [ ] Implement `list_profiles(include_admin: bool) -> Result<Vec<Profile>>`
- [ ] Implement `create_profile(name: &str, linked_email: Option<&str>, is_admin: bool) -> Result<Profile>`
- [ ] Implement `delete_profile(id: i64) -> Result<()>`
- [ ] Implement `get_profile_by_email(email: &str) -> Result<Option<Profile>>`
- [ ] Implement `get_profile(id: i64) -> Result<Option<Profile>>`
- [ ] Implement `subscribe_channel(profile_id: i64, channel_id: i64) -> Result<()>`
- [ ] Implement `unsubscribe_channel(profile_id: i64, channel_id: i64) -> Result<()>`
- [ ] Implement `list_profile_channels(profile_id: i64) -> Result<Vec<i64>>`
- [ ] Implement `ignore_video_for_profile(profile_id: i64, youtube_id: &str) -> Result<()>`
- [ ] Implement `unignore_video_for_profile(profile_id: i64, youtube_id: &str) -> Result<()>`
- [ ] Write unit tests for each new DB method

### Task 2: Profile CRUD routes

- [ ] Create `crates/server/src/routes/profiles.rs`
- [ ] Implement `list_profiles` handler (GET /api/profiles)
- [ ] Implement `create_profile` handler (POST /api/profiles, admin only)
- [ ] Implement `delete_profile` handler (DELETE /api/profiles/{id}, admin only)
- [ ] Implement `get_session` handler (GET /api/profile-session — reads cookie, returns profile)
- [ ] Implement `set_session` handler (POST /api/profile-session — validates profile exists, sets cookie)
- [ ] Implement `clear_session` handler (DELETE /api/profile-session — clears cookie)
- [ ] Implement `list_profile_channels` handler (GET /api/profiles/{id}/channels — profile owner or admin)
- [ ] Implement `subscribe_channel` handler (PUT /api/profiles/{id}/channels/{cid} — profile owner or admin)
- [ ] Implement `unsubscribe_channel` handler (DELETE /api/profiles/{id}/channels/{cid} — profile owner or admin)
- [ ] Register all routes in `lib.rs`

### Task 3: Admin OAuth auto-profile

- [ ] After successful token exchange in `routes/auth.rs`, look up profile by admin email
- [ ] If none exists, create admin profile (`is_admin_profile=1, linked_email=email`)
- [ ] Set `yt_plex_profile` cookie on the response (same path/SameSite as session cookie)

### Task 4: Profile extractor + update channels/videos routes

- [ ] Add `CurrentProfile` Axum extractor that reads `yt_plex_profile` cookie → `Option<i64>`
- [ ] Update `list_channels`: join `profile_channels` when profile is set; admin sees all
- [ ] Update `list_channel_videos`: filter ignores from `profile_video_ignores` for current profile
- [ ] Update `ignore_video`: insert into `profile_video_ignores` for current profile (require profile to be set)
- [ ] Update `unignore_video`: delete from `profile_video_ignores` for current profile

### Task 5: Frontend — `/select-profile` page

- [ ] Create `web/src/routes/select-profile/+page.svelte`
- [ ] Fetch `GET /api/profiles` and render profile cards (name, click to select)
- [ ] On card click: `POST /api/profile-session {profile_id}` then navigate to `/browse`
- [ ] Show "Login as admin" link alongside the profile cards
- [ ] Add `Profile` type and `listProfiles`, `setProfileSession`, `clearProfileSession` to `api.ts`

### Task 6: Frontend — layout guard

- [ ] In `web/src/routes/+layout.svelte` (or `+layout.ts` load function): check for `yt_plex_profile` cookie
- [ ] If missing and route is not `/select-profile` or `/login`, redirect to `/select-profile`
- [ ] Show current profile name in the nav bar (fetch from `/api/profile-session`)
- [ ] Add "Switch profile" link in nav that clears cookie and returns to `/select-profile`

### Task 7: Frontend — channel subscription UI (self-service)

- [ ] Add a channel subscription toggle (subscribe/unsubscribe) on the `/browse` channel grid for each channel card
- [ ] Show only subscribed channels by default; include an "All channels" toggle to browse unsubscribed ones
- [ ] Wire to `PUT`/`DELETE /api/profiles/{id}/channels/{cid}` using the current profile id

### Task 8: Frontend — admin Profiles tab

- [ ] Add "Profiles" tab to `/admin` page
- [ ] List existing non-admin profiles with delete buttons
- [ ] "New profile" form (name input + create button)
- [ ] Wire to profile CRUD endpoints (no channel management needed here)

---

## Notes

- Admin user can always see all channels regardless of subscriptions — no profile filter applied when `is_admin` is true.
- Profile deletion cascades in SQL so ignores and subscriptions are cleaned up automatically.
- The `yt_plex_profile` cookie should be `SameSite=Lax; Path=/` — not `HttpOnly` so the frontend JS can read the profile ID for display without an extra round-trip.
- Migration: no action needed for existing `ignored_at` values — they remain as global suppresses.
