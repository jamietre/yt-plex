# Per-Channel Path Prefix — Design Spec

**Date:** 2026-04-09

## Overview

Add an optional static path prefix per channel so downloaded files land in a subdirectory under `base_path` before the global path template is applied.

Final path: `base_path / path_prefix / template::render(path_template, …)`

If `path_prefix` is NULL or empty the behaviour is identical to today.

---

## Data Model

### DB migration v3
```sql
ALTER TABLE channels ADD COLUMN path_prefix TEXT;
```
Nullable. Default NULL = no prefix. The migration follows the existing pattern in `MIGRATIONS`.

### `Channel` struct (`crates/common/src/models.rs`)
Add field:
```rust
pub path_prefix: Option<String>,
```

---

## Validation

Applied server-side on both add and update. A helper function `validate_path_prefix(s: &str) -> Result<(), &'static str>`:

- Trim leading/trailing whitespace; treat empty string as NULL (no prefix).
- Reject any path segment equal to `..`.
- Reject a string that starts with `/` or `\`.
- Reject null bytes (`\0`) and control characters.
- Allowed characters: anything printable that isn't a path traversal — no explicit whitelist, just the rejections above.

Return `400 Bad Request` with a plain-text description on failure.

---

## API

### `POST /api/channels` — add channel
`AddChannelRequest` gains an optional field:
```rust
pub path_prefix: Option<String>,
```
Validated before insert. Stored as NULL if absent or empty.

### `PUT /api/channels/{id}` — update channel (new endpoint)
```rust
pub struct UpdateChannelRequest {
    pub name: String,
    pub url: String,
    pub path_prefix: Option<String>,
}
```
Admin-only. Validates `path_prefix`. Updates `name`, `youtube_channel_url`, and `path_prefix` on the channel row. Returns the updated `Channel` as JSON.

No migration of existing downloaded files — that is the user's responsibility.

### DB functions (new/changed)
- `insert_channel(url, name, path_prefix)` — adds `path_prefix` parameter
- `update_channel(id, name, url, path_prefix)` — new function
- All `SELECT` queries on `channels` add `path_prefix` to the column list

---

## Path Construction (worker)

After yt-dlp returns metadata the worker already has `meta.channel_id` (the YouTube channel ID, e.g. `UCxxxxxxxxxxxxxxxx`). Use this to look up the channel record:

```rust
let prefix = db.get_channel_by_youtube_id(meta.channel_id.as_deref().unwrap_or(""))
    .ok()
    .flatten()
    .and_then(|ch| ch.path_prefix);

let rel_path = template::render(&path_template, …);
let dest = match prefix.as_deref().filter(|p| !p.is_empty()) {
    Some(p) => PathBuf::from(&base_path).join(p).join(&rel_path),
    None    => PathBuf::from(&base_path).join(&rel_path),
};
```

New DB function required: `get_channel_by_youtube_id(youtube_channel_id: &str) -> Result<Option<Channel>>`.

If the channel isn't found (e.g. raw URL submission for an untracked channel, or channel not yet synced), `prefix` is None and the path is unchanged.

---

## Admin UI

### Files
| File | Action |
|---|---|
| `web/src/lib/components/ChannelForm.svelte` | Create |
| `web/src/lib/api.ts` | Add `updateChannel()` |
| `web/src/routes/admin/+page.svelte` | Replace inline add-row, add Edit buttons, show Prefix column |

### `ChannelForm.svelte`
A side-panel component. Props:
```ts
let { channel, onSave, onCancel }: {
    channel: Channel | null;   // null = add mode, non-null = edit mode
    onSave: (ch: Channel) => void;
    onCancel: () => void;
} = $props();
```

Fields:
- **Display name** (required)
- **YouTube URL** (required)
- **Path prefix** (optional) — with a hint line showing `<base_path> / <prefix> / …` (base_path loaded from the settings already fetched by the admin page). If prefix is empty the hint shows `<base_path> / …`.

On save: calls `addChannel` or `updateChannel` depending on mode. Surfaces API errors inline.

### Admin channels panel changes
- Remove the inline add-row (`<div class="add-row">` with name/URL inputs and Add button).
- Add `"+ Add channel"` button in the panel header (right-aligned).
- Each channel row gets an "Edit" button (secondary, small) alongside Sync / Regen / Remove.
- `ChannelForm` is rendered conditionally in the channels tab, to the right of the table. When open, the table container uses `display: flex` with the form taking a fixed width (~260px) and the table taking the remaining space.
- Channel table gets a **Prefix** column (after Name, before Channel ID) showing the value or `—`.
- State: `editingChannel: Channel | null = null` (non-null = editing), `addingChannel: boolean = false`. Only one of these is true at a time; opening either closes the other.

---

## Out of Scope

- Path prefix does not support template variables (e.g. `{channel_id}`).
- No migration of existing downloaded files when prefix is changed.
- Per-channel user visibility (planned separately).
