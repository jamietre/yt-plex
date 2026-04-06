# yt-plex UX Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restyle the entire SvelteKit frontend with a cinematic media-center aesthetic using a shared design token system and component library, and restructure the admin page as a deep-linkable left-tabbed panel.

**Architecture:** A single `theme.css` file defines all CSS custom properties (design tokens); it is imported once in `+layout.svelte`. Five shared Svelte components (`Button`, `Badge`, `Input`, `EmptyState`, `PageHeader`) are created in `web/src/lib/components/` and used across all pages. Admin tabs are driven by the `?tab=` URL search param so each section is directly addressable.

**Tech Stack:** SvelteKit 5 (runes mode), TypeScript, plain CSS custom properties. No new dependencies. Verification: `cd web && npm run check` (svelte-check + TypeScript). No unit test framework is configured — `npm run check` is the quality gate.

---

## File Map

**Create:**
- `web/src/lib/styles/theme.css` — all CSS custom properties + base body styles
- `web/src/lib/components/Button.svelte` — primary/secondary/danger/ghost variants, md/sm sizes
- `web/src/lib/components/Badge.svelte` — video status badge (new/in_progress/downloaded/ignored)
- `web/src/lib/components/Input.svelte` — text/url/search input with consistent styling
- `web/src/lib/components/EmptyState.svelte` — centred italic empty-list placeholder
- `web/src/lib/components/PageHeader.svelte` — Playfair Display page title + optional subtitle + actions slot

**Modify:**
- `web/src/app.html` — add Google Fonts `<link>` tags
- `web/src/routes/+layout.svelte` — import theme.css; restyle top nav
- `web/src/routes/admin/+page.svelte` — left-tab layout; `?tab=` deep-linking
- `web/src/routes/browse/+page.svelte` — use new components + tokens
- `web/src/routes/browse/[channelId]/+page.svelte` — use new components + tokens
- `web/src/routes/browse/[channelId]/[videoId]/+page.svelte` — use new components + tokens
- `web/src/routes/queue/+page.svelte` — use new components + tokens
- `web/src/routes/select-profile/+page.svelte` — use new components + tokens
- `web/src/routes/login/+page.svelte` — use new components + tokens

**Do not touch:**
- `web/src/routes/settings/+page.svelte` — redirect shim, no changes needed
- All Rust/backend code

---

## Task 1: Design tokens + Google Fonts

**Files:**
- Create: `web/src/lib/styles/theme.css`
- Modify: `web/src/app.html`

- [ ] **Step 1: Create `web/src/lib/styles/theme.css`**

```css
/* yt-plex design tokens */

:root {
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
    --text:       #e8e4dc;
    --text-2:     #9a9490;
    --text-3:     #555050;

    /* Status */
    --green:      #4caf76;
    --orange:     #e8903a;
    --red:        #e05548;

    /* Typography */
    --font-display: 'Playfair Display', Georgia, serif;
    --font-ui:      'Outfit', system-ui, sans-serif;

    /* Shape */
    --radius:    6px;
    --radius-lg: 10px;
}

*, *::before, *::after { box-sizing: border-box; }

html { height: 100%; }

body {
    background: var(--bg);
    color: var(--text);
    font-family: var(--font-ui);
    font-size: 14px;
    line-height: 1.5;
    margin: 0;
    min-height: 100%;
}

a { color: inherit; }

button { font-family: var(--font-ui); }
input, textarea, select { font-family: var(--font-ui); }

code {
    font-size: 0.88em;
    background: var(--surface-2);
    padding: 1px 5px;
    border-radius: 3px;
    color: var(--text-2);
}
```

- [ ] **Step 2: Update `web/src/app.html` to add Google Fonts**

Replace the entire file with:

```html
<!doctype html>
<html lang="en">
	<head>
		<meta charset="utf-8" />
		<meta name="viewport" content="width=device-width, initial-scale=1" />
		<meta name="text-scale" content="scale" />
		<link rel="preconnect" href="https://fonts.googleapis.com" />
		<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
		<link href="https://fonts.googleapis.com/css2?family=Playfair+Display:wght@600;700&family=Outfit:wght@300;400;500;600;700&display=swap" rel="stylesheet" />
		%sveltekit.head%
	</head>
	<body data-sveltekit-preload-data="hover">
		<div style="display: contents">%sveltekit.body%</div>
	</body>
</html>
```

- [ ] **Step 3: Verify**

```bash
cd web && npm run check
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add web/src/lib/styles/theme.css web/src/app.html
git commit -m "feat: add design tokens (theme.css) and Google Fonts"
```

---

## Task 2: Button + Badge components

**Files:**
- Create: `web/src/lib/components/Button.svelte`
- Create: `web/src/lib/components/Badge.svelte`

- [ ] **Step 1: Create `web/src/lib/components/Button.svelte`**

```svelte
<script lang="ts">
    import type { Snippet } from 'svelte';

    type Variant = 'primary' | 'secondary' | 'danger' | 'ghost';
    type Size = 'md' | 'sm';

    let {
        variant = 'secondary',
        size = 'md',
        disabled = false,
        type = 'button',
        onclick,
        children,
    }: {
        variant?: Variant;
        size?: Size;
        disabled?: boolean;
        type?: 'button' | 'submit' | 'reset';
        onclick?: (e: MouseEvent) => void;
        children: Snippet;
    } = $props();
</script>

<button
    {type}
    {disabled}
    class="btn btn-{variant} size-{size}"
    {onclick}
>
    {@render children()}
</button>

<style>
    .btn {
        font-family: var(--font-ui);
        font-weight: 600;
        border-radius: var(--radius);
        cursor: pointer;
        letter-spacing: 0.3px;
        transition: background 0.15s, border-color 0.15s, color 0.15s;
        border: 1px solid transparent;
        line-height: 1;
        white-space: nowrap;
    }
    .size-md { font-size: 13px; padding: 7px 15px; }
    .size-sm { font-size: 11px; padding: 4px 10px; }

    .btn-primary { background: var(--amber); color: #000; border-color: var(--amber); }
    .btn-primary:hover:not(:disabled) { background: var(--amber-glow); border-color: var(--amber-glow); }

    .btn-secondary { background: transparent; border-color: var(--border-2); color: var(--text-2); }
    .btn-secondary:hover:not(:disabled) { border-color: var(--amber); color: var(--amber); }

    .btn-danger { background: transparent; border-color: #5a2020; color: var(--red); }
    .btn-danger:hover:not(:disabled) { border-color: var(--red); }

    .btn-ghost { background: transparent; border-color: transparent; color: var(--text-2); }
    .btn-ghost:hover:not(:disabled) { color: var(--text); }

    .btn:disabled { opacity: 0.45; cursor: default; }
</style>
```

- [ ] **Step 2: Create `web/src/lib/components/Badge.svelte`**

```svelte
<script lang="ts">
    import type { VideoStatus } from '$lib/api';

    let { status }: { status: VideoStatus } = $props();

    const label: Record<VideoStatus, string> = {
        new:         'New',
        in_progress: '↓',
        downloaded:  '✓ Plex',
        ignored:     'Ignored',
    };
</script>

<span class="badge badge-{status}">{label[status]}</span>

<style>
    .badge {
        display: inline-block;
        font-family: var(--font-ui);
        font-size: 9px;
        font-weight: 700;
        padding: 2px 7px;
        border-radius: 3px;
        letter-spacing: 0.5px;
        text-transform: uppercase;
    }
    .badge-new        { background: rgba(232,160,32,0.15); color: var(--amber);  border: 1px solid rgba(232,160,32,0.3); }
    .badge-in_progress{ background: rgba(232,144,58,0.15); color: var(--orange); border: 1px solid rgba(232,144,58,0.3); }
    .badge-downloaded { background: rgba(76,175,118,0.15); color: var(--green);  border: 1px solid rgba(76,175,118,0.3); }
    .badge-ignored    { background: rgba(85,80,80,0.3);    color: var(--text-3); border: 1px solid rgba(85,80,80,0.4);  }
</style>
```

- [ ] **Step 3: Verify**

```bash
cd web && npm run check
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add web/src/lib/components/Button.svelte web/src/lib/components/Badge.svelte
git commit -m "feat: add Button and Badge shared components"
```

---

## Task 3: Input, EmptyState, PageHeader components

**Files:**
- Create: `web/src/lib/components/Input.svelte`
- Create: `web/src/lib/components/EmptyState.svelte`
- Create: `web/src/lib/components/PageHeader.svelte`

- [ ] **Step 1: Create `web/src/lib/components/Input.svelte`**

```svelte
<script lang="ts">
    let {
        type = 'text',
        placeholder = '',
        value = $bindable(''),
        disabled = false,
        class: extraClass = '',
        oninput,
        onkeydown,
    }: {
        type?: 'text' | 'url' | 'search';
        placeholder?: string;
        value?: string;
        disabled?: boolean;
        class?: string;
        oninput?: (e: Event) => void;
        onkeydown?: (e: KeyboardEvent) => void;
    } = $props();
</script>

<input
    {type}
    {placeholder}
    bind:value
    {disabled}
    class="input {extraClass}"
    {oninput}
    {onkeydown}
/>

<style>
    .input {
        font-family: var(--font-ui);
        font-size: 13px;
        background: var(--surface-2);
        border: 1px solid var(--border);
        color: var(--text);
        border-radius: var(--radius);
        padding: 7px 11px;
        outline: none;
        transition: border-color 0.15s;
        width: 100%;
    }
    .input:focus  { border-color: var(--amber); }
    .input::placeholder { color: var(--text-3); }
    .input:disabled { opacity: 0.5; cursor: default; }
</style>
```

- [ ] **Step 2: Create `web/src/lib/components/EmptyState.svelte`**

```svelte
<script lang="ts">
    let { message }: { message: string } = $props();
</script>

<p class="empty">{message}</p>

<style>
    .empty {
        color: var(--text-3);
        font-style: italic;
        font-size: 13px;
        text-align: center;
        padding: 2rem;
        grid-column: 1 / -1;
    }
</style>
```

- [ ] **Step 3: Create `web/src/lib/components/PageHeader.svelte`**

```svelte
<script lang="ts">
    import type { Snippet } from 'svelte';

    let {
        title,
        subtitle,
        actions,
    }: {
        title: string;
        subtitle?: string;
        actions?: Snippet;
    } = $props();
</script>

<div class="page-header">
    <div class="left">
        <h1 class="title">{title}</h1>
        {#if subtitle}
            <span class="subtitle">{subtitle}</span>
        {/if}
    </div>
    {#if actions}
        <div class="actions">
            {@render actions()}
        </div>
    {/if}
</div>

<style>
    .page-header {
        display: flex;
        align-items: baseline;
        gap: 12px;
        margin-bottom: 20px;
    }
    .left { display: flex; align-items: baseline; gap: 10px; flex: 1; min-width: 0; }
    .title {
        font-family: var(--font-display);
        font-size: 22px;
        font-weight: 700;
        color: var(--text);
        line-height: 1.2;
        margin: 0;
    }
    .subtitle {
        font-size: 12px;
        color: var(--text-3);
        flex-shrink: 0;
    }
    .actions { display: flex; align-items: center; gap: 8px; flex-shrink: 0; }
</style>
```

- [ ] **Step 4: Verify**

```bash
cd web && npm run check
```

Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add web/src/lib/components/Input.svelte web/src/lib/components/EmptyState.svelte web/src/lib/components/PageHeader.svelte
git commit -m "feat: add Input, EmptyState, PageHeader shared components"
```

---

## Task 4: Top navigation redesign

**Files:**
- Modify: `web/src/routes/+layout.svelte`

- [ ] **Step 1: Replace `web/src/routes/+layout.svelte`**

Full file replacement (all logic is unchanged; only template markup and styles are updated):

```svelte
<script lang="ts">
    import '$lib/styles/theme.css';
    import { goto } from '$app/navigation';
    import { page } from '$app/stores';
    import { logout, getProfileSession, clearProfileSession, type Profile } from '$lib/api';
    import { onMount } from 'svelte';
    import type { Snippet } from 'svelte';

    let { children }: { children: Snippet } = $props();
    let isAdmin = $state(false);
    let profile = $state<Profile | null>(null);
    let profileLoaded = $state(false);

    const PUBLIC_ROUTES = ['/login', '/select-profile'];

    onMount(async () => {
        const [adminResp, profileData] = await Promise.allSettled([
            fetch('/api/auth/me'),
            getProfileSession(),
        ]);

        if (adminResp.status === 'fulfilled') {
            isAdmin = adminResp.value.ok;
        }
        if (profileData.status === 'fulfilled') {
            profile = profileData.value;
        }
        profileLoaded = true;

        const path = $page.url.pathname;
        if (!profile && !PUBLIC_ROUTES.some(r => path.startsWith(r))) {
            goto('/select-profile');
        }
    });

    async function handleLogout() {
        await logout();
        isAdmin = false;
        window.location.href = '/login';
    }

    async function handleSwitchProfile() {
        await clearProfileSession();
        profile = null;
        goto('/select-profile');
    }

    function isActive(prefix: string) {
        return $page.url.pathname.startsWith(prefix);
    }
</script>

{#if $page.url.pathname !== '/login' && $page.url.pathname !== '/select-profile'}
<nav>
    <a href="/browse" class="logo">yt-plex</a>
    <div class="nav-links">
        <a href="/browse" class:active={isActive('/browse')}>Browse</a>
        <a href="/queue"  class:active={isActive('/queue')}>Queue</a>
        {#if isAdmin}
            <a href="/admin" class:active={isActive('/admin')}>Admin</a>
        {/if}
    </div>
    <div class="nav-right">
        {#if profile}
            <span class="profile-name">{profile.name}</span>
            <button class="btn-switch" onclick={handleSwitchProfile}>Switch</button>
        {/if}
        {#if isAdmin}
            <button class="btn-logout" onclick={handleLogout}>Log out</button>
        {/if}
    </div>
</nav>
{/if}

{@render children()}

<style>
    nav {
        display: flex;
        align-items: center;
        gap: 0;
        background: var(--surface);
        border-bottom: 1px solid var(--border);
        padding: 0 20px;
        height: 48px;
        position: sticky;
        top: 0;
        z-index: 100;
    }

    .logo {
        font-family: var(--font-display);
        font-size: 18px;
        font-weight: 700;
        color: var(--amber);
        text-decoration: none;
        letter-spacing: 0.5px;
        margin-right: 16px;
        flex-shrink: 0;
    }
    .logo:hover { color: var(--amber-glow); }

    .nav-links {
        display: flex;
        align-items: center;
        flex: 1;
    }

    .nav-links a {
        color: var(--text-2);
        text-decoration: none;
        font-size: 13px;
        font-weight: 500;
        padding: 6px 12px;
        border-radius: var(--radius);
        transition: color 0.15s, background 0.15s;
    }
    .nav-links a:hover:not(.active) { color: var(--text); }
    .nav-links a.active {
        color: var(--amber);
        background: rgba(232, 160, 32, 0.08);
    }

    .nav-right {
        display: flex;
        align-items: center;
        gap: 8px;
    }

    .profile-name {
        font-size: 12px;
        color: var(--text-2);
    }

    .btn-switch {
        background: transparent;
        border: 1px solid var(--border-2);
        color: var(--text-2);
        padding: 3px 9px;
        border-radius: var(--radius);
        font-size: 11px;
        font-weight: 500;
        cursor: pointer;
        transition: border-color 0.15s, color 0.15s;
        font-family: var(--font-ui);
    }
    .btn-switch:hover { border-color: var(--amber); color: var(--amber); }

    .btn-logout {
        background: transparent;
        border: 1px solid var(--border-2);
        color: var(--text-2);
        padding: 3px 9px;
        border-radius: var(--radius);
        font-size: 11px;
        font-weight: 500;
        cursor: pointer;
        transition: border-color 0.15s, color 0.15s;
        font-family: var(--font-ui);
    }
    .btn-logout:hover { border-color: var(--red); color: var(--red); }
</style>
```

- [ ] **Step 2: Verify**

```bash
cd web && npm run check
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add web/src/routes/+layout.svelte
git commit -m "feat: restyle top nav with cinematic theme tokens"
```

---

## Task 5: Admin page — left-tab layout + deep-linking

**Files:**
- Modify: `web/src/routes/admin/+page.svelte`

- [ ] **Step 1: Replace `web/src/routes/admin/+page.svelte`**

All business logic (state, handlers) is preserved verbatim; only the template structure and styles change. The `activeTab` is derived from the URL `?tab=` param.

```svelte
<script lang="ts">
    import { page } from '$app/stores';
    import { goto } from '$app/navigation';
    import { onMount, onDestroy } from 'svelte';
    import Button from '$lib/components/Button.svelte';
    import Input from '$lib/components/Input.svelte';
    import EmptyState from '$lib/components/EmptyState.svelte';
    import PageHeader from '$lib/components/PageHeader.svelte';
    import {
        getSettings, updateSettings, type Settings,
        listChannels, addChannel, deleteChannel, syncChannel, rescanFilesystem,
        submitJob, listProfiles, createProfile, deleteProfile,
        type Channel, type Profile,
    } from '$lib/api';

    // Auth guard
    onMount(async () => {
        const res = await fetch('/api/auth/me');
        if (!res.ok) window.location.href = '/login';
    });

    // Active tab (deep-linked via ?tab=)
    const activeTab = $derived($page.url.searchParams.get('tab') ?? 'channels');
    function setTab(tab: string) { goto(`?tab=${tab}`); }

    const tabs = [
        { id: 'channels', label: 'Channels',   icon: '▤' },
        { id: 'profiles', label: 'Profiles',   icon: '◉' },
        { id: 'submit',   label: 'Submit URL',  icon: '⬇' },
        { id: 'settings', label: 'Settings',   icon: '⚙' },
    ] as const;

    // ── Settings ─────────────────────────────────────────────────────────────
    let settings = $state<Settings | null>(null);
    let settingsError = $state('');
    let settingsSaved = $state(false);
    let settingsSaving = $state(false);

    // ── Channels ─────────────────────────────────────────────────────────────
    let channels = $state<Channel[]>([]);
    let newChannelUrl = $state('');
    let newChannelName = $state('');
    let channelError = $state('');
    let addingChannel = $state(false);
    let syncingIds = $state(new Set<string>());
    let pollTimer: ReturnType<typeof setInterval> | null = null;
    let rescanning = $state(false);
    let rescanMsg = $state('');

    // ── URL submission ────────────────────────────────────────────────────────
    let submitUrl = $state('');
    let submitError = $state('');
    let submitSuccess = $state('');
    let submitting = $state(false);

    onMount(async () => {
        try { settings = await getSettings(); } catch { settingsError = 'Failed to load settings'; }
        try { channels = await listChannels(); } catch { /* ignore */ }
    });

    onDestroy(() => { if (pollTimer) clearInterval(pollTimer); });

    async function saveSettings() {
        if (!settings) return;
        settingsSaving = true; settingsError = ''; settingsSaved = false;
        try { await updateSettings(settings); settingsSaved = true; }
        catch (e: unknown) { settingsError = e instanceof Error ? e.message : 'Save failed'; }
        finally { settingsSaving = false; }
    }

    async function handleAddChannel() {
        if (!newChannelUrl || !newChannelName) return;
        addingChannel = true; channelError = '';
        try {
            const ch = await addChannel(newChannelUrl, newChannelName);
            channels = [...channels, ch];
            newChannelUrl = ''; newChannelName = '';
        } catch (e: unknown) {
            channelError = e instanceof Error ? e.message : 'Failed to add channel';
        } finally { addingChannel = false; }
    }

    async function handleDeleteChannel(id: string) {
        if (!confirm('Remove this channel and all its video metadata?')) return;
        try {
            await deleteChannel(id);
            channels = channels.filter(c => c.id !== id);
        } catch { /* ignore */ }
    }

    async function handleRescan() {
        rescanning = true; rescanMsg = '';
        try {
            await rescanFilesystem();
            rescanMsg = 'Re-scan started — downloaded status will update shortly.';
        } catch (e: unknown) {
            rescanMsg = e instanceof Error ? e.message : 'Re-scan failed';
        } finally { rescanning = false; }
    }

    async function handleSyncChannel(id: string) {
        try { await syncChannel(id); } catch { /* ignore */ }
        const before = channels.find(c => c.id === id)?.last_synced_at ?? null;
        syncingIds = new Set([...syncingIds, id]);
        startPolling(id, before);
    }

    function startPolling(id: string, beforeSyncedAt: string | null) {
        if (pollTimer) return;
        pollTimer = setInterval(async () => {
            try {
                const fresh = await listChannels();
                const updated = fresh.find(c => c.id === id);
                if (updated && updated.last_synced_at !== beforeSyncedAt) {
                    channels = fresh;
                    syncingIds = new Set([...syncingIds].filter(x => x !== id));
                }
                if (syncingIds.size === 0) { clearInterval(pollTimer!); pollTimer = null; }
            } catch { /* ignore */ }
        }, 3000);
    }

    // ── Profiles ──────────────────────────────────────────────────────────────
    let profiles = $state<Profile[]>([]);
    let newProfileName = $state('');
    let profileError = $state('');
    let addingProfile = $state(false);

    onMount(async () => {
        try { profiles = await listProfiles(); } catch { /* ignore */ }
    });

    async function handleCreateProfile() {
        if (!newProfileName.trim()) return;
        addingProfile = true; profileError = '';
        try {
            const p = await createProfile(newProfileName.trim());
            profiles = [...profiles, p];
            newProfileName = '';
        } catch (e: unknown) {
            profileError = e instanceof Error ? e.message : 'Failed to create profile';
        } finally { addingProfile = false; }
    }

    async function handleDeleteProfile(id: number, name: string) {
        if (!confirm(`Remove profile "${name}"? This will delete their ignore list and channel subscriptions.`)) return;
        try {
            await deleteProfile(id);
            profiles = profiles.filter(p => p.id !== id);
        } catch (e: unknown) {
            profileError = e instanceof Error ? e.message : 'Failed to delete profile';
        }
    }

    async function handleSubmitUrl() {
        submitError = ''; submitSuccess = '';
        submitting = true;
        try {
            await submitJob(submitUrl);
            submitSuccess = 'Queued!';
            submitUrl = '';
        } catch (e: unknown) {
            submitError = e instanceof Error ? e.message : 'Failed';
        } finally { submitting = false; }
    }
</script>

<div class="admin-layout">
    <aside class="sidebar">
        <span class="sidebar-label">Admin</span>
        {#each tabs as tab}
            <button
                class="sidebar-tab"
                class:active={activeTab === tab.id}
                onclick={() => setTab(tab.id)}
            >
                <span class="tab-icon">{tab.icon}</span>
                {tab.label}
            </button>
        {/each}
    </aside>

    <main class="content">

        {#if activeTab === 'channels'}
            <PageHeader title="Channels" />

            <div class="add-row">
                <Input bind:value={newChannelName} placeholder="Display name" class="name-input" />
                <Input bind:value={newChannelUrl} type="url" placeholder="https://youtube.com/@Channel" class="url-input" />
                <Button variant="primary" size="sm" onclick={handleAddChannel} disabled={addingChannel || !newChannelUrl || !newChannelName}>
                    {addingChannel ? 'Adding…' : 'Add'}
                </Button>
            </div>
            {#if channelError}<p class="msg-error">{channelError}</p>{/if}

            {#if channels.length > 0}
                <table class="data-table">
                    <thead>
                        <tr><th>Name</th><th>Channel ID</th><th>Last synced</th><th></th></tr>
                    </thead>
                    <tbody>
                        {#each channels as ch (ch.id)}
                            {@const syncing = syncingIds.has(ch.id)}
                            <tr class:row-dim={syncing}>
                                <td class="td-primary">{ch.name}</td>
                                <td class="td-mono">{ch.youtube_channel_id ?? '—'}</td>
                                <td>
                                    {#if syncing}
                                        <span class="sync-status">⟳ Syncing…</span>
                                    {:else}
                                        {ch.last_synced_at ? new Date(ch.last_synced_at).toLocaleString() : 'never'}
                                    {/if}
                                </td>
                                <td class="td-actions">
                                    <Button variant="secondary" size="sm" onclick={() => handleSyncChannel(ch.id)} disabled={syncing}>
                                        ↻ Sync
                                    </Button>
                                    <Button variant="danger" size="sm" onclick={() => handleDeleteChannel(ch.id)} disabled={syncing}>
                                        Remove
                                    </Button>
                                </td>
                            </tr>
                        {/each}
                    </tbody>
                </table>
            {:else}
                <EmptyState message="No channels yet." />
            {/if}

            <div class="rescan-row">
                <Button variant="secondary" size="sm" onclick={handleRescan} disabled={rescanning}>
                    {rescanning ? 'Scanning…' : '↺ Re-scan filesystem'}
                </Button>
                <span class="hint">Marks present files as downloaded; clears stale downloaded status for deleted files.</span>
            </div>
            {#if rescanMsg}<p class="msg-ok">{rescanMsg}</p>{/if}

        {:else if activeTab === 'profiles'}
            <PageHeader title="Profiles" />

            <div class="add-row">
                <Input
                    bind:value={newProfileName}
                    placeholder="Profile name"
                    onkeydown={(e) => { if (e.key === 'Enter') handleCreateProfile(); }}
                />
                <Button variant="primary" size="sm" onclick={handleCreateProfile} disabled={addingProfile || !newProfileName.trim()}>
                    {addingProfile ? 'Creating…' : 'Create'}
                </Button>
            </div>
            {#if profileError}<p class="msg-error">{profileError}</p>{/if}

            {#if profiles.length > 0}
                <table class="data-table">
                    <thead><tr><th>Name</th><th>Created</th><th></th></tr></thead>
                    <tbody>
                        {#each profiles as p (p.id)}
                            <tr>
                                <td class="td-primary">{p.name}</td>
                                <td>{new Date(p.created_at).toLocaleDateString()}</td>
                                <td class="td-actions">
                                    <Button variant="danger" size="sm" onclick={() => handleDeleteProfile(p.id, p.name)}>Remove</Button>
                                </td>
                            </tr>
                        {/each}
                    </tbody>
                </table>
            {:else}
                <EmptyState message="No profiles yet." />
            {/if}

        {:else if activeTab === 'submit'}
            <PageHeader title="Submit URL" subtitle="Queue a specific video for download" />

            <div class="add-row">
                <Input type="url" bind:value={submitUrl} placeholder="https://www.youtube.com/watch?v=…" class="url-input" />
                <Button variant="primary" size="sm" onclick={handleSubmitUrl} disabled={submitting || !submitUrl}>
                    {submitting ? 'Queuing…' : 'Queue'}
                </Button>
            </div>
            {#if submitError}<p class="msg-error">{submitError}</p>{/if}
            {#if submitSuccess}<p class="msg-ok">{submitSuccess}</p>{/if}

        {:else if activeTab === 'settings'}
            {#if settings}
                <PageHeader title="Settings" />
                <form onsubmit={(e) => { e.preventDefault(); saveSettings(); }}>
                    <fieldset>
                        <legend>Plex</legend>
                        <label>URL <Input bind:value={settings.plex.url} /></label>
                        <label>Token <Input bind:value={settings.plex.token} /></label>
                        <label>Library Section ID <Input bind:value={settings.plex.library_section_id} /></label>
                    </fieldset>
                    <fieldset>
                        <legend>Output</legend>
                        <label>Base path <Input bind:value={settings.output.base_path} /></label>
                        <label>Path template <Input bind:value={settings.output.path_template} /></label>
                        <label>Thumbnail cache <Input bind:value={settings.output.thumbnail_cache_dir} /></label>
                        <small>
                            Variables: <code>{'{channel}'}</code> channel name,
                            <code>{'{channel_id}'}</code> YouTube channel ID (UCxxxxxxxx),
                            <code>{'{title}'}</code> video title,
                            <code>{'{id}'}</code> YouTube video ID, <code>{'{ext}'}</code> file extension,
                            <code>{'{date}'}</code> full date (YYYY-MM-DD), <code>{'{yyyy}'}</code> year, <code>{'{mm}'}</code> month, <code>{'{dd}'}</code> day.
                            <br />
                            <strong>Important:</strong> the YouTube ID must appear wrapped in square brackets somewhere in the filename
                            (e.g. <code>{'[{id}]'}</code>) so the server can match downloaded files back to their video record.
                            <br />
                            For Plex TV Shows with the <a href="https://github.com/zeroqi/youtube-agent.bundle" target="_blank" rel="noreferrer">YouTube Agent</a>:
                            <code>{'{channel} [{channel_id}]/Season {yyyy}/{title} [{id}].{ext}'}</code>
                        </small>
                    </fieldset>
                    {#if settingsError}<p class="msg-error">{settingsError}</p>{/if}
                    {#if settingsSaved}<p class="msg-ok">Saved.</p>{/if}
                    <Button type="submit" variant="primary" disabled={settingsSaving}>
                        {settingsSaving ? 'Saving…' : 'Save settings'}
                    </Button>
                </form>
            {:else if settingsError}
                <p class="msg-error">{settingsError}</p>
            {/if}
        {/if}

    </main>
</div>

<style>
    .admin-layout {
        display: flex;
        min-height: calc(100vh - 48px);
    }

    /* ── Sidebar ─────────────────────────────────────────────────────────── */
    .sidebar {
        width: 160px;
        flex-shrink: 0;
        background: var(--surface);
        border-right: 1px solid var(--border);
        padding: 16px 8px;
        display: flex;
        flex-direction: column;
        gap: 2px;
    }
    .sidebar-label {
        font-size: 9px;
        text-transform: uppercase;
        letter-spacing: 2px;
        color: var(--text-3);
        font-weight: 700;
        padding: 0 8px;
        margin-bottom: 8px;
    }
    .sidebar-tab {
        display: flex;
        align-items: center;
        gap: 8px;
        padding: 8px 10px;
        border-radius: 5px;
        font-size: 12px;
        font-weight: 500;
        color: var(--text-2);
        background: transparent;
        border: none;
        cursor: pointer;
        text-align: left;
        transition: background 0.12s, color 0.12s;
        font-family: var(--font-ui);
        width: 100%;
    }
    .sidebar-tab:hover { background: var(--surface-2); color: var(--text); }
    .sidebar-tab.active { background: rgba(232,160,32,0.1); color: var(--amber); }
    .tab-icon { font-size: 13px; width: 16px; text-align: center; flex-shrink: 0; }

    /* ── Content ──────────────────────────────────────────────────────────── */
    .content {
        flex: 1;
        padding: 28px 32px;
        overflow-y: auto;
        max-width: 860px;
    }

    /* ── Add row ──────────────────────────────────────────────────────────── */
    .add-row {
        display: flex;
        gap: 8px;
        margin-bottom: 12px;
        align-items: center;
        flex-wrap: wrap;
    }
    .add-row :global(.url-input) { flex: 1; min-width: 200px; }
    .add-row :global(.name-input) { width: 160px; flex-shrink: 0; }

    /* ── Table ────────────────────────────────────────────────────────────── */
    .data-table {
        width: 100%;
        border-collapse: collapse;
        font-size: 12px;
        margin-bottom: 16px;
    }
    .data-table th {
        text-align: left;
        padding: 6px 10px;
        color: var(--text-3);
        border-bottom: 1px solid var(--border);
        font-size: 10px;
        font-weight: 700;
        text-transform: uppercase;
        letter-spacing: 0.5px;
    }
    .data-table td {
        padding: 8px 10px;
        border-bottom: 1px solid var(--border);
        color: var(--text-2);
        vertical-align: middle;
    }
    .td-primary { color: var(--text); font-weight: 500; }
    .td-mono { font-family: monospace; font-size: 11px; color: var(--text-3); }
    .td-actions { display: flex; gap: 6px; white-space: nowrap; }
    .row-dim td { opacity: 0.5; }
    .sync-status { color: var(--orange); font-size: 11px; }

    /* ── Rescan ───────────────────────────────────────────────────────────── */
    .rescan-row {
        display: flex;
        align-items: center;
        gap: 12px;
        margin-top: 8px;
        flex-wrap: wrap;
    }
    .hint { font-size: 11px; color: var(--text-3); }

    /* ── Messages ─────────────────────────────────────────────────────────── */
    .msg-error { color: var(--red);   font-size: 12px; margin: 6px 0; }
    .msg-ok    { color: var(--green); font-size: 12px; margin: 6px 0; }

    /* ── Form ─────────────────────────────────────────────────────────────── */
    form { display: flex; flex-direction: column; gap: 16px; max-width: 540px; }
    fieldset {
        border: 1px solid var(--border);
        border-radius: var(--radius);
        padding: 14px 16px;
        display: flex;
        flex-direction: column;
        gap: 10px;
    }
    legend { color: var(--text-2); font-size: 11px; font-weight: 600; padding: 0 6px; text-transform: uppercase; letter-spacing: 1px; }
    label { display: flex; flex-direction: column; gap: 5px; font-size: 12px; color: var(--text-2); font-weight: 500; }
    small { color: var(--text-3); font-size: 11px; line-height: 1.7; }
    small a { color: var(--amber); }
    strong { color: var(--text-2); }
</style>
```

- [ ] **Step 2: Verify**

```bash
cd web && npm run check
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add web/src/routes/admin/+page.svelte
git commit -m "feat: admin page left-tab layout with ?tab= deep-linking"
```

---

## Task 6: Browse — channel grid

**Files:**
- Modify: `web/src/routes/browse/+page.svelte`

- [ ] **Step 1: Replace `web/src/routes/browse/+page.svelte`**

```svelte
<script lang="ts">
    import { onMount } from 'svelte';
    import {
        listChannels, listAllChannels,
        subscribeChannel, unsubscribeChannel, getProfileSession,
        type Channel,
    } from '$lib/api';
    import PageHeader from '$lib/components/PageHeader.svelte';
    import Button from '$lib/components/Button.svelte';
    import EmptyState from '$lib/components/EmptyState.svelte';

    let channels = $state<Channel[]>([]);
    let allChannels = $state<Channel[]>([]);
    let subscribedIds = $state(new Set<string>());
    let error = $state('');
    let showAll = $state(false);
    let profileId = $state<number | null>(null);
    let toggling = $state<string | null>(null);

    onMount(async () => {
        try {
            const [session, subbed] = await Promise.all([
                getProfileSession(),
                listChannels(),
            ]);
            profileId = session?.id ?? null;
            channels = subbed;
            subscribedIds = new Set(subbed.map(c => c.id));
            if (profileId) {
                allChannels = await listAllChannels();
            }
        } catch (e: unknown) {
            error = e instanceof Error ? e.message : 'Failed to load channels';
        }
    });

    async function toggleSubscription(channel: Channel) {
        if (!profileId) return;
        toggling = channel.id;
        try {
            if (subscribedIds.has(channel.id)) {
                await unsubscribeChannel(profileId, channel.id);
                subscribedIds.delete(channel.id);
                subscribedIds = new Set(subscribedIds);
                channels = channels.filter(c => c.id !== channel.id);
            } else {
                await subscribeChannel(profileId, channel.id);
                subscribedIds.add(channel.id);
                subscribedIds = new Set(subscribedIds);
                channels = [...channels, channel].sort((a, b) => a.name.localeCompare(b.name));
            }
        } catch (e: unknown) {
            error = e instanceof Error ? e.message : 'Failed to update subscription';
        } finally {
            toggling = null;
        }
    }

    function timeAgo(isoString: string | null): string {
        if (!isoString) return 'never';
        const diff = Date.now() - new Date(isoString).getTime();
        const hours = Math.floor(diff / 3600000);
        if (hours < 1) return 'just now';
        if (hours < 24) return `${hours}h ago`;
        return `${Math.floor(hours / 24)}d ago`;
    }

    const displayChannels = $derived(showAll ? allChannels : channels);
    const channelCountText = $derived(`${displayChannels.length} channel${displayChannels.length === 1 ? '' : 's'}`);
</script>

<div class="page">
    {#if error}
        <p class="msg-error">{error}</p>
    {:else}
        <PageHeader title="Your Channels" subtitle={channelCountText}>
            {#snippet actions()}
                {#if profileId && allChannels.length > 0}
                    <label class="show-all-toggle">
                        <input
                            type="checkbox"
                            checked={showAll}
                            onchange={() => { showAll = !showAll; }}
                        />
                        Show all
                    </label>
                {/if}
            {/snippet}
        </PageHeader>

        {#if displayChannels.length === 0 && !showAll}
            <EmptyState message="You haven't subscribed to any channels yet. Toggle 'Show all' to find and subscribe." />
        {:else}
            <div class="grid">
                {#each displayChannels as channel (channel.id)}
                    <div class="card-wrap">
                        <a href="/browse/{channel.id}" class="card" class:unsubscribed={!subscribedIds.has(channel.id)}>
                            <div class="card-name">{channel.name}</div>
                            <div class="card-meta">Synced {timeAgo(channel.last_synced_at)}</div>
                        </a>
                        {#if profileId && showAll}
                            <Button
                                variant={subscribedIds.has(channel.id) ? 'secondary' : 'primary'}
                                size="sm"
                                disabled={toggling === channel.id}
                                onclick={(e) => { e.preventDefault(); toggleSubscription(channel); }}
                            >
                                {#if toggling === channel.id}
                                    …
                                {:else if subscribedIds.has(channel.id)}
                                    ✓ Subscribed
                                {:else}
                                    + Subscribe
                                {/if}
                            </Button>
                        {/if}
                    </div>
                {/each}
            </div>
        {/if}
    {/if}
</div>

<style>
    .page { padding: 28px 24px; }

    .show-all-toggle {
        display: flex;
        align-items: center;
        gap: 6px;
        font-size: 12px;
        color: var(--text-2);
        cursor: pointer;
        user-select: none;
    }
    .show-all-toggle input { accent-color: var(--amber); width: 13px; height: 13px; }

    .grid {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(190px, 1fr));
        gap: 10px;
    }
    .card-wrap { display: flex; flex-direction: column; gap: 5px; }

    .card {
        display: block;
        background: var(--surface-2);
        border: 1px solid var(--border);
        border-radius: var(--radius);
        padding: 14px 14px 12px;
        text-decoration: none;
        transition: border-color 0.15s, background 0.15s;
    }
    .card:hover { border-color: var(--amber); background: var(--surface-3); }
    .card.unsubscribed { opacity: 0.4; }

    .card-name { font-size: 13px; font-weight: 600; color: var(--text); margin-bottom: 4px; }
    .card-meta { font-size: 11px; color: var(--text-3); }

    .msg-error { color: var(--red); font-size: 13px; padding: 28px 24px; }
</style>
```

- [ ] **Step 2: Verify**

```bash
cd web && npm run check
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add web/src/routes/browse/+page.svelte
git commit -m "feat: restyle browse channel grid"
```

---

## Task 7: Browse — video grid

**Files:**
- Modify: `web/src/routes/browse/[channelId]/+page.svelte`

- [ ] **Step 1: Replace `web/src/routes/browse/[channelId]/+page.svelte`**

```svelte
<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { page } from '$app/stores';
    import {
        listChannels, listVideos, ignoreVideo, unignoreVideo,
        submitJobByYoutubeId, syncChannel,
        type Channel, type Video, type VideoStatus
    } from '$lib/api';
    import { createWsStore } from '$lib/ws';
    import Badge from '$lib/components/Badge.svelte';
    import Button from '$lib/components/Button.svelte';
    import EmptyState from '$lib/components/EmptyState.svelte';
    import PageHeader from '$lib/components/PageHeader.svelte';

    const channelId = $derived($page.params.channelId);

    let channel = $state<Channel | null>(null);
    let videos = $state<Video[]>([]);
    let filter = $state<'new' | 'downloaded' | 'all'>('new');
    let showIgnored = $state(false);
    let search = $state('');
    let error = $state('');
    let syncing = $state(false);
    let loading = $state(false);
    let hasMore = $state(false);
    let offset = $state(0);
    const LIMIT = 48;

    let selected = $state(new Set<string>());
    let bulkWorking = $state(false);

    const ws = createWsStore();
    let unsubWs: (() => void) | undefined;

    let searchTimer: ReturnType<typeof setTimeout> | null = null;
    let sentinel: HTMLDivElement | undefined;
    let observer: IntersectionObserver | null = null;

    async function loadPage(reset: boolean) {
        if (loading) return;
        loading = true;
        const currentOffset = reset ? 0 : offset;
        try {
            const result = await listVideos(channelId, filter, showIgnored, search, LIMIT, currentOffset);
            if (reset) {
                videos = result.videos;
                selected = new Set();
            } else {
                videos = [...videos, ...result.videos];
            }
            hasMore = result.has_more;
            offset = currentOffset + result.videos.length;
        } catch (e: unknown) {
            error = e instanceof Error ? e.message : 'Failed to load videos';
        } finally {
            loading = false;
        }
    }

    function resetAndLoad() { offset = 0; hasMore = false; loadPage(true); }

    onMount(async () => {
        try {
            const channels = await listChannels();
            channel = channels.find(c => c.id === channelId) ?? null;
        } catch { /* ignore */ }
        await loadPage(true);
        ws.connect();
        unsubWs = ws.subscribe(() => {});

        observer = new IntersectionObserver((entries) => {
            if (entries[0].isIntersecting && hasMore && !loading) loadPage(false);
        }, { rootMargin: '200px' });
        if (sentinel) observer.observe(sentinel);
    });

    onDestroy(() => {
        ws.disconnect();
        unsubWs?.();
        observer?.disconnect();
        if (searchTimer) clearTimeout(searchTimer);
    });

    $effect(() => {
        const msg = $ws;
        if (!msg?.youtube_id) return;
        videos = videos.map(v => {
            if (v.youtube_id !== msg.youtube_id) return v;
            const newStatus: VideoStatus =
                msg.status === 'done' ? 'downloaded'
                : (msg.status === 'queued' || msg.status === 'downloading' || msg.status === 'copying') ? 'in_progress'
                : v.status;
            return { ...v, status: newStatus };
        });
    });

    function handleSearchInput(e: Event) {
        search = (e.target as HTMLInputElement).value;
        if (searchTimer) clearTimeout(searchTimer);
        searchTimer = setTimeout(resetAndLoad, 300);
    }

    async function handleDownload(youtubeId: string) {
        try {
            await submitJobByYoutubeId(youtubeId);
            videos = videos.map(v =>
                v.youtube_id === youtubeId ? { ...v, status: 'in_progress' as VideoStatus } : v
            );
        } catch (e: unknown) {
            alert(e instanceof Error ? e.message : 'Failed to queue download');
        }
    }

    async function handleIgnore(youtubeId: string) {
        try {
            await ignoreVideo(youtubeId);
            videos = videos.map(v =>
                v.youtube_id === youtubeId ? { ...v, status: 'ignored' as VideoStatus, ignored_at: new Date().toISOString() } : v
            );
            selected = new Set([...selected].filter(id => id !== youtubeId));
        } catch { /* ignore */ }
    }

    async function handleUnignore(youtubeId: string) {
        try {
            await unignoreVideo(youtubeId);
            videos = videos.map(v =>
                v.youtube_id === youtubeId ? { ...v, status: 'new' as VideoStatus, ignored_at: null } : v
            );
        } catch { /* ignore */ }
    }

    async function handleSync() {
        syncing = true;
        try { await syncChannel(channelId); setTimeout(resetAndLoad, 2000); }
        catch { /* ignore */ } finally { syncing = false; }
    }

    function toggleSelect(youtubeId: string) {
        const next = new Set(selected);
        if (next.has(youtubeId)) next.delete(youtubeId); else next.add(youtubeId);
        selected = next;
    }

    async function bulkDownload() {
        bulkWorking = true;
        for (const id of selected) {
            const v = videos.find(v => v.youtube_id === id);
            if (v?.status === 'new') await handleDownload(id).catch(() => {});
        }
        selected = new Set();
        bulkWorking = false;
    }

    async function bulkIgnore() {
        bulkWorking = true;
        for (const id of selected) { await handleIgnore(id).catch(() => {}); }
        selected = new Set();
        bulkWorking = false;
    }
</script>

<div class="page">
    <a href="/browse" class="back-link">← Channels</a>

    <PageHeader title={channel?.name ?? channelId}>
        {#snippet actions()}
            <Button variant="secondary" size="sm" onclick={handleSync} disabled={syncing}>
                {syncing ? 'Syncing…' : '↻ Refresh'}
            </Button>
        {/snippet}
    </PageHeader>

    <div class="toolbar">
        <div class="filters">
            <span class="filter-label">Show:</span>
            {#each (['new', 'downloaded', 'all'] as const) as f}
                <button
                    class="pill"
                    class:active={filter === f}
                    onclick={() => { filter = f; resetAndLoad(); }}
                >{f}</button>
            {/each}
            <label class="toggle-label">
                <input type="checkbox" checked={showIgnored} onchange={() => { showIgnored = !showIgnored; resetAndLoad(); }} />
                Ignored
            </label>
        </div>
        <input
            class="search-input"
            type="search"
            placeholder="Search titles…"
            value={search}
            oninput={handleSearchInput}
        />
    </div>

    {#if selected.size > 0}
        <div class="bulk-bar">
            <span class="bulk-count">{selected.size} selected</span>
            <Button variant="secondary" size="sm" onclick={bulkDownload} disabled={bulkWorking}>↓ Download all</Button>
            <Button variant="danger" size="sm" onclick={bulkIgnore} disabled={bulkWorking}>✕ Ignore all</Button>
            <Button variant="ghost" size="sm" onclick={() => selected = new Set()}>Clear</Button>
        </div>
    {/if}

    {#if error}<p class="msg-error">{error}</p>{/if}

    <div class="grid">
        {#each videos as video (video.youtube_id)}
            {@const isSelected = selected.has(video.youtube_id)}
            <div class="card" class:card-selected={isSelected}>
                <label class="check-wrap">
                    <input
                        type="checkbox"
                        class="card-check"
                        checked={isSelected}
                        onchange={() => toggleSelect(video.youtube_id)}
                    />
                </label>
                <a href="/browse/{channelId}/{video.youtube_id}" class="thumb-link">
                    <div class="thumb">
                        <img src="/api/thumbnails/{video.youtube_id}" alt={video.title} loading="lazy" />
                        <span class="thumb-badge"><Badge status={video.status} /></span>
                    </div>
                </a>
                <div class="card-body">
                    <a href="/browse/{channelId}/{video.youtube_id}" class="card-title" title={video.title}>
                        {video.title}
                    </a>
                    <div class="card-actions">
                        {#if video.status === 'new'}
                            <button class="btn-dl" onclick={() => handleDownload(video.youtube_id)}>Download</button>
                            <button class="btn-ign" onclick={() => handleIgnore(video.youtube_id)}>✕</button>
                        {:else if video.status === 'in_progress'}
                            <button class="btn-state state-progress" disabled>Queued…</button>
                        {:else if video.status === 'downloaded'}
                            <button class="btn-state state-done" disabled>On Plex ✓</button>
                        {:else if video.status === 'ignored'}
                            <button class="btn-ign" onclick={() => handleUnignore(video.youtube_id)}>Unignore</button>
                        {/if}
                    </div>
                </div>
            </div>
        {/each}
        {#if videos.length === 0 && !loading && !error}
            <EmptyState message="No videos match this filter." />
        {/if}
    </div>

    {#if loading}<p class="loading-msg">Loading…</p>{/if}
    <div bind:this={sentinel} class="sentinel"></div>
</div>

<style>
    .page { padding: 20px 24px; }

    .back-link {
        display: inline-block;
        font-size: 12px;
        color: var(--text-3);
        text-decoration: none;
        margin-bottom: 12px;
        transition: color 0.15s;
    }
    .back-link:hover { color: var(--amber); }

    /* Toolbar */
    .toolbar {
        display: flex;
        align-items: center;
        gap: 10px;
        margin-bottom: 14px;
        flex-wrap: wrap;
    }
    .filters { display: flex; align-items: center; gap: 5px; flex-wrap: wrap; }
    .filter-label { font-size: 11px; color: var(--text-3); margin-right: 2px; }
    .pill {
        background: var(--surface-2);
        color: var(--text-2);
        border: 1px solid var(--border);
        border-radius: 20px;
        padding: 4px 12px;
        font-size: 11px;
        font-weight: 500;
        cursor: pointer;
        transition: all 0.12s;
        font-family: var(--font-ui);
    }
    .pill.active { background: rgba(232,160,32,0.15); color: var(--amber); border-color: rgba(232,160,32,0.4); }
    .toggle-label {
        display: flex;
        align-items: center;
        gap: 4px;
        font-size: 11px;
        color: var(--text-3);
        cursor: pointer;
        margin-left: 4px;
    }
    .toggle-label input { accent-color: var(--amber); }
    .search-input {
        margin-left: auto;
        padding: 5px 12px;
        background: var(--surface-2);
        border: 1px solid var(--border);
        color: var(--text);
        border-radius: 20px;
        font-size: 12px;
        outline: none;
        font-family: var(--font-ui);
        min-width: 180px;
        transition: border-color 0.15s;
    }
    .search-input:focus { border-color: var(--amber); }
    .search-input::placeholder { color: var(--text-3); }

    /* Bulk bar */
    .bulk-bar {
        display: flex;
        align-items: center;
        gap: 8px;
        background: var(--surface-2);
        border: 1px solid rgba(232,160,32,0.3);
        border-radius: var(--radius);
        padding: 6px 12px;
        margin-bottom: 12px;
    }
    .bulk-count { font-size: 12px; color: var(--text-2); margin-right: auto; }

    /* Grid */
    .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(175px, 1fr)); gap: 8px; }

    .card {
        background: var(--surface-2);
        border: 1px solid var(--border);
        border-radius: var(--radius);
        overflow: hidden;
        position: relative;
        transition: border-color 0.15s;
    }
    .card:hover { border-color: var(--border-2); }
    .card-selected { border-color: var(--amber) !important; box-shadow: 0 0 0 1px var(--amber-dim); }

    .check-wrap { position: absolute; top: 5px; left: 5px; z-index: 2; cursor: pointer; }
    .card-check { width: 15px; height: 15px; accent-color: var(--amber); cursor: pointer; }

    .thumb-link { display: block; text-decoration: none; }
    .thumb { position: relative; }
    .thumb img {
        width: 100%;
        aspect-ratio: 16/9;
        object-fit: cover;
        display: block;
        background: var(--surface-3);
    }
    .thumb-badge { position: absolute; top: 5px; right: 5px; }

    .card-body { padding: 7px 9px 8px; }
    .card-title {
        font-size: 11px;
        color: var(--text);
        line-height: 1.35;
        display: -webkit-box;
        -webkit-line-clamp: 2;
        -webkit-box-orient: vertical;
        overflow: hidden;
        text-decoration: none;
        margin-bottom: 6px;
        display: block;
    }
    .card-title:hover { color: var(--amber); }

    .card-actions { display: flex; gap: 4px; }
    .btn-dl {
        flex: 1;
        background: var(--amber);
        color: #000;
        border: none;
        border-radius: 3px;
        padding: 3px 0;
        font-size: 9px;
        font-weight: 700;
        cursor: pointer;
        font-family: var(--font-ui);
        transition: background 0.12s;
    }
    .btn-dl:hover { background: var(--amber-glow); }
    .btn-ign {
        background: var(--surface-3);
        color: var(--text-3);
        border: none;
        border-radius: 3px;
        padding: 3px 7px;
        font-size: 9px;
        cursor: pointer;
        font-family: var(--font-ui);
        transition: color 0.12s;
    }
    .btn-ign:hover { color: var(--red); }
    .btn-state {
        flex: 1;
        background: var(--surface-3);
        border: 1px solid var(--border);
        border-radius: 3px;
        padding: 3px 0;
        font-size: 9px;
        cursor: default;
        font-family: var(--font-ui);
    }
    .state-progress { color: var(--orange); border-color: rgba(232,144,58,0.3); }
    .state-done     { color: var(--green);  border-color: rgba(76,175,118,0.3); }

    .msg-error { color: var(--red); font-size: 13px; margin-bottom: 12px; }
    .loading-msg { color: var(--text-3); font-style: italic; font-size: 13px; text-align: center; padding: 1rem; }
    .sentinel { height: 1px; }
</style>
```

- [ ] **Step 2: Verify**

```bash
cd web && npm run check
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add web/src/routes/browse/[channelId]/+page.svelte
git commit -m "feat: restyle video grid with cinematic theme"
```

---

## Task 8: Video detail page

**Files:**
- Modify: `web/src/routes/browse/[channelId]/[videoId]/+page.svelte`

- [ ] **Step 1: Replace `web/src/routes/browse/[channelId]/[videoId]/+page.svelte`**

```svelte
<script lang="ts">
    import { onMount } from 'svelte';
    import { page } from '$app/stores';
    import {
        getVideo, submitJobByYoutubeId, ignoreVideo, unignoreVideo,
        type Video, type VideoStatus
    } from '$lib/api';
    import Badge from '$lib/components/Badge.svelte';
    import Button from '$lib/components/Button.svelte';

    const channelId = $derived($page.params.channelId);
    const videoId   = $derived($page.params.videoId);

    let video = $state<Video | null>(null);
    let loading = $state(true);
    let error = $state('');
    let actionWorking = $state(false);
    let actionMsg = $state('');
    let isAdmin = $state(false);

    onMount(async () => {
        const [videoResult] = await Promise.allSettled([
            getVideo(videoId),
            fetch('/api/auth/me').then(r => { isAdmin = r.ok; }),
        ]);
        if (videoResult.status === 'fulfilled') {
            video = videoResult.value;
        } else {
            error = videoResult.reason instanceof Error ? videoResult.reason.message : 'Failed to load video';
        }
        loading = false;
    });

    async function handleDownload() {
        if (!video) return;
        actionWorking = true; actionMsg = '';
        try {
            await submitJobByYoutubeId(video.youtube_id);
            video = { ...video, status: 'in_progress' as VideoStatus };
            actionMsg = 'Queued for download!';
        } catch (e: unknown) {
            actionMsg = e instanceof Error ? e.message : 'Failed';
        } finally { actionWorking = false; }
    }

    async function handleIgnore() {
        if (!video) return;
        actionWorking = true;
        try {
            await ignoreVideo(video.youtube_id);
            video = { ...video, status: 'ignored' as VideoStatus, ignored_at: new Date().toISOString() };
        } catch { /* ignore */ } finally { actionWorking = false; }
    }

    async function handleUnignore() {
        if (!video) return;
        actionWorking = true;
        try {
            await unignoreVideo(video.youtube_id);
            video = { ...video, status: 'new' as VideoStatus, ignored_at: null };
        } catch { /* ignore */ } finally { actionWorking = false; }
    }
</script>

<div class="page">
    <a href="/browse/{channelId}" class="back-link">← Back to channel</a>

    {#if loading}
        <p class="hint">Loading…</p>
    {:else if error}
        <p class="msg-error">{error}</p>
    {:else if video}
        <div class="detail">
            <img class="thumb" src="/api/thumbnails/{video.youtube_id}" alt={video.title} />
            <div class="info">
                <h1 class="title">{video.title}</h1>
                <p class="meta">
                    {#if video.published_at}
                        Added to YouTube: {new Date(video.published_at).toLocaleDateString()}
                    {:else}
                        First seen: {new Date(video.last_seen_at).toLocaleDateString()}
                    {/if}
                </p>
                <div class="status-row">
                    <Badge status={video.status} />
                </div>
                {#if video.file_path}
                    <p class="file-path" title={video.file_path}>{video.file_path}</p>
                {/if}
                <div class="actions">
                    {#if video.status === 'new'}
                        <Button variant="primary" onclick={handleDownload} disabled={actionWorking}>↓ Download</Button>
                        <Button variant="secondary" onclick={handleIgnore} disabled={actionWorking}>Ignore</Button>
                    {:else if video.status === 'in_progress'}
                        <Button variant="secondary" disabled>Queued…</Button>
                    {:else if video.status === 'downloaded'}
                        <Button variant="secondary" disabled>On Plex ✓</Button>
                    {:else if video.status === 'ignored'}
                        <Button variant="secondary" onclick={handleUnignore} disabled={actionWorking}>Unignore</Button>
                    {/if}
                </div>
                {#if actionMsg}<p class="action-msg">{actionMsg}</p>{/if}
                {#if isAdmin}
                    <a class="yt-link" href="https://www.youtube.com/watch?v={video.youtube_id}" target="_blank" rel="noreferrer">
                        Watch on YouTube ↗
                    </a>
                {/if}
            </div>
        </div>

        {#if video.description}
            <section class="description">
                <h2 class="desc-heading">Description</h2>
                <pre class="desc-text">{video.description}</pre>
            </section>
        {/if}
    {/if}
</div>

<style>
    .page { max-width: 860px; padding: 20px 24px; }

    .back-link {
        display: inline-block;
        font-size: 12px;
        color: var(--text-3);
        text-decoration: none;
        margin-bottom: 16px;
        transition: color 0.15s;
    }
    .back-link:hover { color: var(--amber); }

    .hint     { color: var(--text-3); font-style: italic; font-size: 13px; }
    .msg-error{ color: var(--red);    font-size: 13px; }

    .detail { display: flex; gap: 24px; margin-bottom: 24px; flex-wrap: wrap; }
    .thumb  {
        width: 320px;
        max-width: 100%;
        border-radius: var(--radius);
        flex-shrink: 0;
        background: var(--surface-3);
        display: block;
    }
    .info { flex: 1; min-width: 200px; }

    .title {
        font-family: var(--font-display);
        font-size: 20px;
        font-weight: 700;
        color: var(--text);
        margin: 0 0 6px;
        line-height: 1.3;
    }
    .meta        { font-size: 11px; color: var(--text-3); margin: 0 0 10px; }
    .status-row  { margin-bottom: 10px; }
    .file-path   { font-size: 11px; color: var(--text-3); font-family: monospace; word-break: break-all; margin: 0 0 10px; }
    .actions     { display: flex; gap: 8px; flex-wrap: wrap; margin-bottom: 8px; }
    .action-msg  { font-size: 12px; color: var(--green); margin: 0; }
    .yt-link     { font-size: 12px; color: var(--amber); text-decoration: none; }
    .yt-link:hover { text-decoration: underline; }

    .description   { border-top: 1px solid var(--border); padding-top: 16px; }
    .desc-heading  { font-size: 14px; font-weight: 600; color: var(--text-2); margin: 0 0 8px; }
    .desc-text {
        white-space: pre-wrap;
        word-break: break-word;
        font-family: var(--font-ui);
        font-size: 13px;
        color: var(--text-2);
        line-height: 1.65;
        margin: 0;
        max-height: 400px;
        overflow-y: auto;
    }
</style>
```

- [ ] **Step 2: Verify**

```bash
cd web && npm run check
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add web/src/routes/browse/[channelId]/[videoId]/+page.svelte
git commit -m "feat: restyle video detail page"
```

---

## Task 9: Queue page

**Files:**
- Modify: `web/src/routes/queue/+page.svelte`

- [ ] **Step 1: Replace `web/src/routes/queue/+page.svelte`**

```svelte
<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { listJobs, submitJob, type Job } from '$lib/api';
    import { createWsStore } from '$lib/ws';
    import Badge from '$lib/components/Badge.svelte';
    import Button from '$lib/components/Button.svelte';
    import PageHeader from '$lib/components/PageHeader.svelte';
    import EmptyState from '$lib/components/EmptyState.svelte';

    let jobs = $state<Job[]>([]);
    let url = $state('');
    let submitError = $state('');
    let submitting = $state(false);

    const ws = createWsStore();

    onMount(async () => {
        try { jobs = await listJobs(); } catch { /* ignore */ }
        ws.connect();
    });
    onDestroy(() => ws.disconnect());

    let unsubWs: (() => void) | undefined;
    onMount(() => { unsubWs = ws.subscribe(() => {}); });
    onDestroy(() => unsubWs?.());

    $effect(() => {
        const msg = $ws;
        if (msg) {
            jobs = jobs.map(j =>
                j.id === msg.job_id
                    ? {
                        ...j,
                        status: msg.status,
                        channel_name: msg.channel_name ?? j.channel_name,
                        title: msg.title ?? j.title,
                        error: msg.error,
                        progress: msg.progress ?? (msg.status !== 'downloading' ? null : j.progress),
                      }
                    : j
            );
        }
    });

    async function handleSubmit() {
        submitError = '';
        submitting = true;
        try {
            const job = await submitJob(url);
            jobs = [job, ...jobs];
            url = '';
        } catch (e: unknown) {
            submitError = e instanceof Error ? e.message : 'Submit failed';
        } finally {
            submitting = false;
        }
    }

    // Map Job status → VideoStatus for Badge (only new/in_progress/downloaded map cleanly;
    // queued/copying map to in_progress; failed gets a custom inline display)
    function jobStatusBadge(status: Job['status']): 'new' | 'in_progress' | 'downloaded' | 'ignored' {
        if (status === 'done')       return 'downloaded';
        if (status === 'failed')     return 'ignored';
        return 'in_progress';
    }

    const statusLabel: Record<Job['status'], string> = {
        queued:      'Queued',
        downloading: 'Downloading',
        copying:     'Copying',
        done:        'Done',
        failed:      'Failed',
    };
</script>

<div class="page">
    <PageHeader title="Download Queue" />

    <form onsubmit={(e) => { e.preventDefault(); handleSubmit(); }} class="submit-form">
        <input
            type="url"
            bind:value={url}
            placeholder="https://www.youtube.com/watch?v=…"
            disabled={submitting}
            class="url-input"
        />
        <Button type="submit" variant="primary" disabled={submitting || !url}>
            {submitting ? 'Queuing…' : 'Add URL'}
        </Button>
    </form>
    {#if submitError}<p class="msg-error">{submitError}</p>{/if}

    {#if jobs.length === 0}
        <EmptyState message="No downloads yet." />
    {:else}
        <table class="data-table">
            <thead>
                <tr>
                    <th>Status</th>
                    <th>Channel</th>
                    <th>Title</th>
                    <th>Added</th>
                </tr>
            </thead>
            <tbody>
                {#each jobs as job (job.id)}
                    <tr>
                        <td class="td-status">
                            <Badge status={jobStatusBadge(job.status)} />
                            {#if job.status === 'downloading' && job.progress != null}
                                <span class="progress">{job.progress.toFixed(0)}%</span>
                            {/if}
                            {#if job.status !== 'done' && job.status !== 'failed'}
                                <span class="status-label">{statusLabel[job.status]}</span>
                            {/if}
                            {#if job.error}
                                <span class="error-indicator" title={job.error}>⚠</span>
                            {/if}
                        </td>
                        <td>{job.channel_name ?? '—'}</td>
                        <td class="td-title">{job.title ?? '—'}</td>
                        <td class="td-date">{new Date(job.created_at).toLocaleString()}</td>
                    </tr>
                {/each}
            </tbody>
        </table>
    {/if}
</div>

<style>
    .page { padding: 28px 24px; max-width: 900px; }

    .submit-form {
        display: flex;
        gap: 8px;
        margin-bottom: 8px;
        align-items: center;
    }
    .url-input {
        flex: 1;
        font-family: var(--font-ui);
        font-size: 13px;
        background: var(--surface-2);
        border: 1px solid var(--border);
        color: var(--text);
        border-radius: var(--radius);
        padding: 7px 11px;
        outline: none;
        transition: border-color 0.15s;
    }
    .url-input:focus { border-color: var(--amber); }
    .url-input::placeholder { color: var(--text-3); }
    .url-input:disabled { opacity: 0.5; }

    .msg-error { color: var(--red); font-size: 12px; margin: 0 0 16px; }

    .data-table {
        width: 100%;
        border-collapse: collapse;
        font-size: 12px;
        margin-top: 20px;
    }
    .data-table th {
        text-align: left;
        padding: 6px 10px;
        color: var(--text-3);
        border-bottom: 1px solid var(--border);
        font-size: 10px;
        font-weight: 700;
        text-transform: uppercase;
        letter-spacing: 0.5px;
    }
    .data-table td {
        padding: 9px 10px;
        border-bottom: 1px solid var(--border);
        color: var(--text-2);
        vertical-align: middle;
    }

    .td-status { display: flex; align-items: center; gap: 6px; white-space: nowrap; }
    .status-label { font-size: 11px; color: var(--text-3); }
    .progress { font-size: 11px; color: var(--orange); font-weight: 600; }
    .error-indicator { color: var(--red); font-size: 13px; cursor: help; }

    .td-title { color: var(--text); max-width: 340px; }
    .td-date  { white-space: nowrap; font-size: 11px; color: var(--text-3); }
</style>
```

- [ ] **Step 2: Verify**

```bash
cd web && npm run check
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add web/src/routes/queue/+page.svelte
git commit -m "feat: restyle queue page"
```

---

## Task 10: Select profile + login pages

**Files:**
- Modify: `web/src/routes/select-profile/+page.svelte`
- Modify: `web/src/routes/login/+page.svelte`

- [ ] **Step 1: Replace `web/src/routes/select-profile/+page.svelte`**

```svelte
<script lang="ts">
    import { goto } from '$app/navigation';
    import { listProfiles, setProfileSession, getAdminProfile, type Profile } from '$lib/api';
    import { onMount } from 'svelte';

    let profiles = $state<Profile[]>([]);
    let adminProfile = $state<Profile | null>(null);
    let loading = $state(true);
    let error = $state('');
    let selecting = $state<number | null>(null);

    onMount(async () => {
        try {
            [profiles, adminProfile] = await Promise.all([listProfiles(), getAdminProfile()]);
        } catch (e) {
            error = e instanceof Error ? e.message : 'Failed to load profiles';
        } finally {
            loading = false;
        }
    });

    async function selectProfile(profile: Profile) {
        selecting = profile.id;
        try {
            await setProfileSession(profile.id);
            goto('/browse');
        } catch (e) {
            error = e instanceof Error ? e.message : 'Failed to select profile';
            selecting = null;
        }
    }
</script>

<div class="page">
    <div class="card">
        <h1 class="heading">Who's watching?</h1>

        {#if loading}
            <p class="hint">Loading profiles…</p>
        {:else if error}
            <p class="msg-error">{error}</p>
        {:else if profiles.length === 0 && !adminProfile}
            <p class="hint">No profiles yet. Ask an admin to create one, or log in as admin below.</p>
        {:else}
            <div class="grid">
                {#each profiles as profile}
                    <button
                        class="profile-btn"
                        disabled={selecting !== null}
                        onclick={() => selectProfile(profile)}
                    >
                        <div class="avatar">{profile.name[0].toUpperCase()}</div>
                        <span class="profile-name">{profile.name}</span>
                        {#if selecting === profile.id}
                            <span class="selecting">…</span>
                        {/if}
                    </button>
                {/each}
                {#if adminProfile}
                    <button
                        class="profile-btn admin-btn"
                        disabled={selecting !== null}
                        onclick={() => selectProfile(adminProfile!)}
                    >
                        <div class="avatar admin-avatar">⚙</div>
                        <span class="profile-name">Admin</span>
                        {#if selecting === adminProfile.id}
                            <span class="selecting">…</span>
                        {/if}
                    </button>
                {/if}
            </div>
        {/if}

        {#if !adminProfile}
            <div class="login-row">
                <a href="/login" class="login-link">Admin login ↗</a>
            </div>
        {/if}
    </div>
</div>

<style>
    .page {
        min-height: 100vh;
        display: flex;
        align-items: center;
        justify-content: center;
        padding: 2rem;
    }
    .card {
        width: 100%;
        max-width: 560px;
        text-align: center;
    }
    .heading {
        font-family: var(--font-display);
        font-size: 28px;
        font-weight: 700;
        color: var(--text);
        margin: 0 0 2rem;
    }
    .hint     { color: var(--text-2); font-size: 13px; font-style: italic; }
    .msg-error{ color: var(--red);    font-size: 13px; }

    .grid {
        display: flex;
        flex-wrap: wrap;
        gap: 12px;
        justify-content: center;
        margin-bottom: 2rem;
    }

    .profile-btn {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 8px;
        background: var(--surface-2);
        border: 1px solid var(--border);
        border-radius: var(--radius-lg);
        padding: 20px 24px;
        cursor: pointer;
        min-width: 120px;
        transition: border-color 0.15s, background 0.15s;
        font-family: var(--font-ui);
    }
    .profile-btn:hover:not(:disabled) { background: var(--surface-3); border-color: var(--amber); }
    .profile-btn:disabled { opacity: 0.6; cursor: default; }

    .avatar {
        width: 52px;
        height: 52px;
        border-radius: 50%;
        background: var(--amber);
        color: #000;
        font-size: 1.3rem;
        font-weight: 700;
        display: flex;
        align-items: center;
        justify-content: center;
        font-family: var(--font-display);
    }
    .admin-btn { border-color: var(--border-2); }
    .admin-btn:hover:not(:disabled) { border-color: var(--orange); }
    .admin-avatar { background: var(--surface-3); color: var(--text-2); font-size: 1.1rem; border: 1px solid var(--border-2); }

    .profile-name { color: var(--text); font-size: 13px; font-weight: 500; }
    .selecting    { color: var(--text-3); font-size: 12px; }

    .login-row { margin-top: 1.5rem; }
    .login-link { color: var(--text-3); font-size: 12px; text-decoration: none; transition: color 0.15s; }
    .login-link:hover { color: var(--amber); }
</style>
```

- [ ] **Step 2: Replace `web/src/routes/login/+page.svelte`**

```svelte
<script lang="ts">
    import { startDeviceLogin, pollDeviceAuth, type DeviceLoginResponse } from '$lib/api';
    import Button from '$lib/components/Button.svelte';

    type Phase = 'idle' | 'waiting' | 'done' | 'error';

    let phase = $state<Phase>('idle');
    let deviceInfo = $state<DeviceLoginResponse | null>(null);
    let errorMsg = $state('');
    let pollTimer: ReturnType<typeof setTimeout> | null = null;
    let networkErrors = 0;
    const MAX_NETWORK_ERRORS = 5;

    function cancelPoll() {
        if (pollTimer !== null) { clearTimeout(pollTimer); pollTimer = null; }
    }

    async function startLogin() {
        cancelPoll();
        phase = 'idle'; errorMsg = ''; deviceInfo = null; networkErrors = 0;
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
            networkErrors = 0;
            if (result.status === 'pending') {
                schedulePoll(result.interval ?? deviceInfo.interval);
            } else if (result.status === 'done') {
                phase = 'done';
                window.location.href = '/';
            } else if (result.status === 'expired') {
                phase = 'error'; errorMsg = 'Sign-in timed out. Please try again.';
            } else {
                phase = 'error'; errorMsg = result.message ?? 'Sign-in failed. Please try again.';
            }
        } catch {
            networkErrors++;
            if (networkErrors >= MAX_NETWORK_ERRORS) {
                phase = 'error'; errorMsg = 'Lost connection to server. Please try again.';
            } else {
                schedulePoll(deviceInfo.interval);
            }
        }
    }
</script>

<div class="page">
    <div class="card">
        <h1 class="heading">yt-plex</h1>
        <p class="sub">Admin sign-in</p>

        {#if phase === 'idle' || phase === 'error'}
            {#if errorMsg}<p class="msg-error">{errorMsg}</p>{/if}
            <Button variant="primary" onclick={startLogin}>Sign in with Google</Button>

        {:else if phase === 'waiting' && deviceInfo}
            <p class="instruction">Click the link below to sign in:</p>
            <a
                class="auth-link"
                href="{deviceInfo.verification_url}?user_code={deviceInfo.user_code}"
                target="_blank"
                rel="noreferrer"
            >
                Sign in with Google ↗
            </a>
            <div class="user-code">{deviceInfo.user_code}</div>
            <p class="hint">Waiting for authorisation…</p>

        {:else if phase === 'done'}
            <p class="hint">Signed in! Redirecting…</p>
        {/if}
    </div>
</div>

<style>
    .page {
        min-height: 100vh;
        display: flex;
        align-items: center;
        justify-content: center;
        padding: 2rem;
    }
    .card {
        width: 100%;
        max-width: 380px;
        background: var(--surface);
        border: 1px solid var(--border);
        border-radius: var(--radius-lg);
        padding: 2.5rem 2rem;
        text-align: center;
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 12px;
    }
    .heading {
        font-family: var(--font-display);
        font-size: 32px;
        font-weight: 700;
        color: var(--amber);
        margin: 0;
        letter-spacing: 1px;
    }
    .sub { font-size: 12px; color: var(--text-3); margin: 0; }

    .msg-error   { color: var(--red);    font-size: 13px; }
    .instruction { color: var(--text-2); font-size: 13px; margin: 0; }
    .hint        { color: var(--text-3); font-size: 12px; margin: 0; }

    .auth-link {
        color: var(--amber);
        font-size: 14px;
        font-weight: 500;
        text-decoration: none;
    }
    .auth-link:hover { text-decoration: underline; }

    .user-code {
        font-size: 2rem;
        font-weight: 700;
        letter-spacing: 0.25em;
        color: var(--text);
        font-family: monospace;
        background: var(--surface-2);
        padding: 8px 20px;
        border-radius: var(--radius);
        border: 1px solid var(--border);
    }
</style>
```

- [ ] **Step 3: Verify**

```bash
cd web && npm run check
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add web/src/routes/select-profile/+page.svelte web/src/routes/login/+page.svelte
git commit -m "feat: restyle select-profile and login pages"
```

---

## Done

All 10 tasks complete. Start the dev server to visually verify the full redesign:

```bash
mise run dev
```

Open http://localhost:5173 (or whichever port Vite reports). Check:
- Top nav: amber `yt-plex` logo, active tab highlight
- Browse: Playfair Display heading, channel cards
- Video grid: status badges on thumbnails, pill filters, amber Download button
- Admin: left sidebar tabs, navigate to `/admin?tab=settings` directly
- Queue: table with badge status column
- Select profile: centred card layout
- Login: centred dark card with amber heading
