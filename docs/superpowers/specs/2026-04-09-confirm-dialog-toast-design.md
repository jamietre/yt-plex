# Confirm Dialog & Toast — Design Spec

**Date:** 2026-04-09

## Overview

Replace browser-native `confirm()` and `alert()` calls with themed, reusable in-app components that follow the app's design tokens.

## Components

### 1. `web/src/lib/confirm.ts`

A tiny store that drives the confirm dialog programmatically. Exposes a single `confirm(options)` function that returns a `Promise<boolean>`.

```ts
confirm({ title: 'Regen metadata?', message: 'Re-fetch .info.json for all downloaded videos without redownloading.' })
// resolves true (confirmed) or false (cancelled)
```

Internally holds a single piece of nullable state: `{ title, message, confirmLabel?, cancelLabel?, resolve }`. Only one dialog can be open at a time.

### 2. `web/src/lib/components/ConfirmDialog.svelte`

Modal overlay component. Mounted once in `+layout.svelte`. Subscribes to the confirm store and renders when state is non-null.

**Appearance:**
- Full-viewport semi-transparent backdrop (`rgba(0,0,0,0.65)`)
- Centred card: `var(--surface-2)` background, `var(--border-2)` border, `var(--radius-lg)` corners, drop shadow
- Title in `var(--text)`, message in `var(--text-2)`
- Actions: Cancel (ghost button) + Confirm (primary/amber button), right-aligned
- Default labels: "Cancel" / "Confirm" — overridable per call

**Behaviour:**
- Clicking backdrop resolves `false` (cancel)
- Pressing Escape resolves `false`
- Confirm button resolves `true`
- Cancel button resolves `false`
- Dialog is removed from DOM after resolution

### 3. `web/src/lib/toast.ts`

Writable store of active toasts. Each toast: `{ id, message, variant, duration }`.

```ts
toast('42 videos queued')               // success (default)
toast('Something failed', { variant: 'error' })
```

`variant` options: `'success'` (default) | `'error'` | `'info'`  
`duration` default: 3500ms  

Exported `toast()` function pushes to the store and schedules removal after `duration`.

### 4. `web/src/lib/components/Toaster.svelte`

Renders active toasts. Mounted once in `+layout.svelte`.

**Appearance (style C — amber tint):**
- Fixed, bottom-right, `z-index` above everything
- Each toast: amber-tinted background (`#1e1608`), amber border (`#3d2e0a`), `var(--radius-lg)` corners
- Icon (✓ / ✕) in `var(--amber)` or `var(--red)` depending on variant
- Title in `var(--amber-glow)` for success, `var(--red)` for error
- Slides in from the right, fades out on dismiss
- Multiple toasts stack vertically with a small gap

## Integration

Replace in `web/src/routes/admin/+page.svelte`:

| Before | After |
|---|---|
| `if (!confirm('Remove this channel…'))` | `if (!await confirm({ title: 'Remove channel?', message: '…' }))` |
| `alert('Regenerating metadata for N videos…')` | `toast('Regenerating metadata for N videos')` |

Add `<ConfirmDialog />` and `<Toaster />` to `web/src/routes/+layout.svelte`.

## Files Touched

- `web/src/lib/confirm.ts` — new
- `web/src/lib/components/ConfirmDialog.svelte` — new
- `web/src/lib/toast.ts` — new
- `web/src/lib/components/Toaster.svelte` — new
- `web/src/routes/+layout.svelte` — add two component mounts
- `web/src/routes/admin/+page.svelte` — replace `confirm()`/`alert()` calls
