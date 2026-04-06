# yt-plex UX Redesign — Design Spec

**Date:** 2026-04-06  
**Status:** Approved

---

## Goals

1. Consistent, uniform styling across all pages via a shared design token system and component library.
2. Reusable Svelte components for simpler long-term maintenance.
3. Admin page restructured as a left-tabbed panel with deep-linkable tabs.
4. All pages and admin tab sections deep-linkable (browser back/forward works, URLs are shareable).

---

## Aesthetic Direction

**Cinematic Media Center** — inspired by Plex and film culture. Deep blacks, warm amber/gold accent, serif display headings on a clean sans-serif UI font. Premium, content-focused feel suited to a home media library.

---

## Design Tokens

Single file: `web/src/lib/styles/theme.css`  
Imported once in `web/src/app.html` via `<link>`.

```css
/* Surfaces */
--bg:         #0a0a0a;
--surface:    #111111;
--surface-2:  #181818;
--surface-3:  #1e1e1e;

/* Borders */
--border:     #2a2a2a;
--border-2:   #333333;

/* Accent */
--amber:      #e8a020;
--amber-dim:  #b87a18;
--amber-glow: #f0b840;

/* Text */
--text:       #e8e4dc;   /* primary */
--text-2:     #9a9490;   /* secondary */
--text-3:     #555050;   /* muted / disabled */

/* Status colours */
--green:      #4caf76;   /* downloaded / on Plex */
--orange:     #e8903a;   /* in progress */
--red:        #e05548;   /* error / danger */

/* Typography */
--font-display: 'Playfair Display', Georgia, serif;
--font-ui:      'Outfit', system-ui, sans-serif;

/* Shape */
--radius:    6px;
--radius-lg: 10px;
```

Google Fonts loaded via `<link>` tags in `app.html`: `Playfair Display` (600, 700) and `Outfit` (300, 400, 500, 600, 700).

`theme.css` is imported in `+layout.svelte` via `import '$lib/styles/theme.css'` in the `<script>` block — this is the correct Vite/SvelteKit approach for a global stylesheet.

---

## Typography Scale

| Role | Font | Size | Weight |
|---|---|---|---|
| Page title / channel name | Playfair Display | 22–28px | 700 |
| Section heading | Playfair Display | 17–20px | 600 |
| Nav links, labels, buttons | Outfit | 12–14px | 500–600 |
| Body / video titles | Outfit | 13px | 400 |
| Meta / timestamps | Outfit | 11px | 400 |
| Micro labels (badges, table headers) | Outfit | 9–10px | 600–700 |

---

## Shared Component Library

Location: `web/src/lib/components/`

### `Button.svelte`
Props: `variant` (`primary` | `secondary` | `danger` | `ghost`), `size` (`md` | `sm`), `disabled`, `onclick`.

- **primary**: amber fill, black text — main CTAs (Download, Add, Save)
- **secondary**: transparent, dim border — sync, refresh actions
- **danger**: transparent, red border/text — destructive actions (Remove)
- **ghost**: no border/background — cancel, back links

### `Badge.svelte`
Props: `status` (`new` | `in_progress` | `downloaded` | `ignored`).

Rendered as a small pill with amber/orange/green/muted colouring. Used on video thumbnails and in tables.

### `Input.svelte`
Props: `type` (`text` | `url` | `search`), `placeholder`, `value`, `disabled`.

Consistent styling: `--surface-2` background, `--border` outline, amber focus ring.

### `EmptyState.svelte`
Props: `message`.

Centred italic muted text for empty list states.

### `PageHeader.svelte`
Props: `title`, optional `subtitle`, optional `actions` slot.

Renders the Playfair Display page title with optional sub-count and right-aligned action slot.

---

## Navigation (`+layout.svelte`)

Top bar, `--surface` background, 1px `--border` bottom.

- Left: `yt-plex` logo in Playfair Display, amber, links to `/browse`
- Nav links: Browse, Queue, Admin (admin-only) — amber active state with subtle amber tint background
- Right: profile name + Switch button (amber ghost); admin logout

No changes to existing route structure. All existing paths (`/browse`, `/queue`, `/admin`, `/select-profile`, `/login`) remain as-is.

---

## Admin Page (`/admin`)

### Layout
Two-column: fixed 160px left sidebar + flexible content panel.

```
┌─────────────────────────────────────────────┐
│ top nav                                     │
├──────────┬──────────────────────────────────┤
│ sidebar  │  content panel                  │
│          │                                  │
│ Channels │  <active tab content>           │
│ Profiles │                                  │
│ Submit   │                                  │
│ Settings │                                  │
└──────────┴──────────────────────────────────┘
```

### Deep-linking
Active tab driven by `?tab=` URL search param. Default (no param) = `channels`.

| Tab | URL |
|---|---|
| Channels | `/admin?tab=channels` (default) |
| Profiles | `/admin?tab=profiles` |
| Submit URL | `/admin?tab=submit` |
| Settings | `/admin?tab=settings` |

SvelteKit `$page.url.searchParams.get('tab')` reads the active tab. Clicking a sidebar item calls `goto('?tab=X')` — no full page reload, browser history preserved.

### Tabs content (unchanged functionality, new styling)
- **Channels**: add-channel form, channels table with sync/remove actions, re-scan button
- **Profiles**: create-profile form, profiles table with remove
- **Submit URL**: single URL input + queue button
- **Settings**: Plex config fieldset + Output config fieldset + save button

---

## Browse — Channel Grid (`/browse`)

- `PageHeader` with title "Your Channels" and subscription count subtitle
- "Show all" toggle (checkbox) in header right slot
- `auto-fill` CSS grid, `minmax(180px, 1fr)` columns, `--surface-2` cards
- Each card: channel name (Outfit 600), meta line (synced X ago), amber badge when there are new videos
- Unsubscribed channels (when "Show all" is on): 40% opacity with `+ Subscribe` button below card
- Deep-link: `/browse` (no change needed — already a stable path)

---

## Browse — Video Grid (`/browse/[channelId]`)

- Back link `← Channels` (ghost style), `PageHeader` with channel name + Refresh button in actions slot
- Toolbar: pill filter buttons (New / Downloaded / All) + Show ignored checkbox + search input (right-aligned)
- Bulk action bar (shown when items selected): amber-tinted bar with Download all / Ignore all / Clear
- `auto-fill` CSS grid, `minmax(175px, 1fr)` columns
- Each card: 16:9 thumbnail, `Badge` overlaid top-right, title (2-line clamp), action buttons
- Deep-link: `/browse/[channelId]` — already a stable path; filter/search state is ephemeral (not persisted to URL, acceptable for this use case)

---

## Video Detail (`/browse/[channelId]/[videoId]`)

Existing functionality preserved. Styled with design tokens: page header, consistent button variants, badge for status.

---

## Queue (`/queue`)

- `PageHeader` "Download Queue"
- Consistent table using `--border` row separators, `--text-2` column headers
- Status column uses `Badge` component
- Progress shown inline next to "Downloading" badge (e.g. `↓ 42%`)
- URL submit form at top: `Input` + `Button` primary

---

## Select Profile (`/select-profile`)

Centred layout. Profile name cards in a grid using the same card style as channel grid. Amber hover border.

---

## Login (`/login`)

Centred card layout on `--bg`. Playfair Display heading, device-flow instructions, amber CTA button.

---

## File Structure Changes

```
web/src/
  app.html                          ← add Google Fonts link + theme.css import
  lib/
    styles/
      theme.css                     ← new: all CSS custom properties + base resets
    components/
      Button.svelte                 ← new
      Badge.svelte                  ← new
      Input.svelte                  ← new
      EmptyState.svelte             ← new
      PageHeader.svelte             ← new
  routes/
    +layout.svelte                  ← updated: nav styling
    browse/+page.svelte             ← updated: use new components + tokens
    browse/[channelId]/+page.svelte ← updated: use new components + tokens
    browse/[channelId]/[videoId]/+page.svelte ← updated: styling
    queue/+page.svelte              ← updated: use new components + tokens
    admin/+page.svelte              ← updated: left-tab layout + ?tab= deep-linking
    select-profile/+page.svelte    ← updated: styling
    login/+page.svelte             ← updated: styling
```

---

## Deep-linking Summary

| Feature | Mechanism |
|---|---|
| Admin tabs | `?tab=channels` query param (default: channels) |
| Channel pages | `/browse/[channelId]` — path segment (existing) |
| Video detail | `/browse/[channelId]/[videoId]` — path segment (existing) |
| Filter/search state | Ephemeral (not in URL) — acceptable for this use case |

---

## Files Not Modified

- `web/src/routes/settings/+page.svelte` — this is a redirect shim (`goto('/admin')`) and requires no changes.
- All backend Rust code — no API or data model changes.

---

## Out of Scope

- Dark/light mode toggle
- Mobile/responsive breakpoints (beyond what naturally works)
- Animations beyond CSS transitions already present
- Changes to API, backend, or data model
