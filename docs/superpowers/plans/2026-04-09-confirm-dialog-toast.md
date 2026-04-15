# Confirm Dialog & Toast Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace all browser `confirm()` and `alert()` calls with themed in-app components — a modal confirmation dialog and an amber-tinted toast notification.

**Architecture:** A `confirm.ts` store exposes an async `confirm()` function that resolves a `Promise<boolean>`; `ConfirmDialog.svelte` subscribes to it and renders when active. A `toast.ts` store holds a list of active toasts; `Toaster.svelte` renders them fixed bottom-right and auto-dismisses each after 3.5s. Both are mounted once in `+layout.svelte`.

**Tech Stack:** SvelteKit 5 (runes mode), TypeScript, CSS custom properties from `theme.css`

---

## File Map

| File | Action | Purpose |
|---|---|---|
| `web/src/lib/confirm.ts` | Create | Store + async `confirm()` function |
| `web/src/lib/components/ConfirmDialog.svelte` | Create | Modal overlay, subscribes to confirm store |
| `web/src/lib/toast.ts` | Create | Store + `toast()` push function |
| `web/src/lib/components/Toaster.svelte` | Create | Fixed bottom-right toast renderer |
| `web/src/routes/+layout.svelte` | Modify | Mount `<ConfirmDialog />` and `<Toaster />` |
| `web/src/routes/admin/+page.svelte` | Modify | Replace `confirm()` / `alert()` calls |

---

## Task 1: `confirm.ts` — programmatic confirm store

**Files:**
- Create: `web/src/lib/confirm.ts`

- [ ] **Step 1: Create the store**

```ts
// web/src/lib/confirm.ts
import { writable } from 'svelte/store';

export interface ConfirmOptions {
    title: string;
    message: string;
    confirmLabel?: string;
    cancelLabel?: string;
}

interface ConfirmState extends ConfirmOptions {
    resolve: (value: boolean) => void;
}

export const confirmState = writable<ConfirmState | null>(null);

export function confirm(options: ConfirmOptions): Promise<boolean> {
    return new Promise((resolve) => {
        confirmState.set({ ...options, resolve });
    });
}
```

- [ ] **Step 2: Commit**

```bash
git add web/src/lib/confirm.ts
git commit -m "feat: add confirm store"
```

---

## Task 2: `ConfirmDialog.svelte` — modal overlay

**Files:**
- Create: `web/src/lib/components/ConfirmDialog.svelte`

- [ ] **Step 1: Create the component**

```svelte
<!-- web/src/lib/components/ConfirmDialog.svelte -->
<script lang="ts">
    import { confirmState } from '$lib/confirm';

    function respond(value: boolean) {
        confirmState.update(s => {
            s?.resolve(value);
            return null;
        });
    }

    function onKeydown(e: KeyboardEvent) {
        if (e.key === 'Escape') respond(false);
    }
</script>

<svelte:window onkeydown={onKeydown} />

{#if $confirmState}
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="backdrop" onclick={() => respond(false)}>
        <div class="dialog" role="dialog" aria-modal="true" onclick={(e) => e.stopPropagation()}>
            <p class="title">{$confirmState.title}</p>
            <p class="message">{$confirmState.message}</p>
            <div class="actions">
                <button class="btn-cancel" onclick={() => respond(false)}>
                    {$confirmState.cancelLabel ?? 'Cancel'}
                </button>
                <button class="btn-confirm" onclick={() => respond(true)}>
                    {$confirmState.confirmLabel ?? 'Confirm'}
                </button>
            </div>
        </div>
    </div>
{/if}

<style>
    .backdrop {
        position: fixed;
        inset: 0;
        background: rgba(0, 0, 0, 0.65);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 1000;
    }

    .dialog {
        background: var(--surface-2);
        border: 1px solid var(--border-2);
        border-radius: var(--radius-lg);
        padding: 24px;
        width: 320px;
        box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
    }

    .title {
        font-size: 15px;
        font-weight: 600;
        color: var(--text);
        margin: 0 0 8px;
    }

    .message {
        font-size: 13px;
        color: var(--text-2);
        line-height: 1.5;
        margin: 0 0 20px;
    }

    .actions {
        display: flex;
        gap: 8px;
        justify-content: flex-end;
    }

    .btn-cancel, .btn-confirm {
        font-family: var(--font-ui);
        font-weight: 600;
        font-size: 12px;
        padding: 5px 14px;
        border-radius: var(--radius);
        cursor: pointer;
        border: 1px solid transparent;
        line-height: 1;
        transition: background 0.15s, border-color 0.15s, color 0.15s;
    }

    .btn-cancel {
        background: transparent;
        border-color: transparent;
        color: var(--text-2);
    }
    .btn-cancel:hover { color: var(--text); }

    .btn-confirm {
        background: var(--amber);
        border-color: var(--amber);
        color: #000;
    }
    .btn-confirm:hover { background: var(--amber-glow); border-color: var(--amber-glow); }
</style>
```

- [ ] **Step 2: Commit**

```bash
git add web/src/lib/components/ConfirmDialog.svelte
git commit -m "feat: add ConfirmDialog component"
```

---

## Task 3: `toast.ts` — toast store

**Files:**
- Create: `web/src/lib/toast.ts`

- [ ] **Step 1: Create the store**

```ts
// web/src/lib/toast.ts
import { writable } from 'svelte/store';

export type ToastVariant = 'success' | 'error' | 'info';

export interface Toast {
    id: number;
    message: string;
    variant: ToastVariant;
}

export const toasts = writable<Toast[]>([]);

let nextId = 0;

export function toast(message: string, options?: { variant?: ToastVariant; duration?: number }) {
    const id = nextId++;
    const variant = options?.variant ?? 'success';
    const duration = options?.duration ?? 3500;

    toasts.update(ts => [...ts, { id, message, variant }]);

    setTimeout(() => {
        toasts.update(ts => ts.filter(t => t.id !== id));
    }, duration);
}
```

- [ ] **Step 2: Commit**

```bash
git add web/src/lib/toast.ts
git commit -m "feat: add toast store"
```

---

## Task 4: `Toaster.svelte` — toast renderer

**Files:**
- Create: `web/src/lib/components/Toaster.svelte`

- [ ] **Step 1: Create the component**

```svelte
<!-- web/src/lib/components/Toaster.svelte -->
<script lang="ts">
    import { toasts } from '$lib/toast';
</script>

{#if $toasts.length > 0}
    <div class="toaster">
        {#each $toasts as t (t.id)}
            <div class="toast toast-{t.variant}">
                <span class="icon">
                    {#if t.variant === 'error'}✕{:else}✓{/if}
                </span>
                <span class="message">{t.message}</span>
            </div>
        {/each}
    </div>
{/if}

<style>
    .toaster {
        position: fixed;
        bottom: 20px;
        right: 20px;
        display: flex;
        flex-direction: column;
        gap: 8px;
        z-index: 1100;
        pointer-events: none;
    }

    .toast {
        display: flex;
        align-items: center;
        gap: 10px;
        padding: 11px 16px;
        border-radius: var(--radius-lg);
        font-family: var(--font-ui);
        font-size: 13px;
        box-shadow: 0 4px 20px rgba(0, 0, 0, 0.5);
        animation: slide-in 0.2s ease;
    }

    .toast-success {
        background: #1e1608;
        border: 1px solid #3d2e0a;
        color: var(--amber-glow);
    }

    .toast-error {
        background: #1e0808;
        border: 1px solid #3d0a0a;
        color: var(--red);
    }

    .toast-info {
        background: var(--surface-2);
        border: 1px solid var(--border-2);
        color: var(--text-2);
    }

    .icon {
        font-size: 14px;
        flex-shrink: 0;
    }

    .toast-success .icon { color: var(--amber); }
    .toast-error .icon   { color: var(--red); }
    .toast-info .icon    { color: var(--text-3); }

    @keyframes slide-in {
        from { opacity: 0; transform: translateX(16px); }
        to   { opacity: 1; transform: translateX(0); }
    }
</style>
```

- [ ] **Step 2: Commit**

```bash
git add web/src/lib/components/Toaster.svelte
git commit -m "feat: add Toaster component"
```

---

## Task 5: Mount in layout

**Files:**
- Modify: `web/src/routes/+layout.svelte`

- [ ] **Step 1: Import and mount both components**

Add to the `<script>` block (after the existing imports):

```ts
import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
import Toaster from '$lib/components/Toaster.svelte';
```

Add after `{@render children()}` (before `<style>`):

```svelte
<ConfirmDialog />
<Toaster />
```

- [ ] **Step 2: Verify the layout file looks like this around the render call**

```svelte
{@render children()}

<ConfirmDialog />
<Toaster />

<style>
```

- [ ] **Step 3: Commit**

```bash
git add web/src/routes/+layout.svelte
git commit -m "feat: mount ConfirmDialog and Toaster in layout"
```

---

## Task 6: Replace browser dialogs in admin page

**Files:**
- Modify: `web/src/routes/admin/+page.svelte`

- [ ] **Step 1: Update imports**

Replace:
```ts
        listChannels, addChannel, deleteChannel, syncChannel, rescanFilesystem, regenChannelMetadata,
        submitJob, listProfiles, createProfile, deleteProfile,
        type Channel, type Profile,
    } from '$lib/api';
```

Add after the `} from '$lib/api';` line:
```ts
    import { confirm } from '$lib/confirm';
    import { toast } from '$lib/toast';
```

- [ ] **Step 2: Replace `handleDeleteChannel`**

Find:
```ts
    async function handleDeleteChannel(id: string) {
        if (!confirm('Remove this channel and all its video metadata?')) return;
        try {
            await deleteChannel(id);
```

Replace with:
```ts
    async function handleDeleteChannel(id: string) {
        const ok = await confirm({
            title: 'Remove channel?',
            message: 'This removes the channel and all its video metadata. Downloaded files are not deleted.',
            confirmLabel: 'Remove',
        });
        if (!ok) return;
        try {
            await deleteChannel(id);
```

- [ ] **Step 3: Replace `handleRegenMetadata`**

Find:
```ts
    async function handleRegenMetadata(id: string) {
        regenningIds = new Set([...regenningIds, id]);
        try {
            const { queued } = await regenChannelMetadata(id);
            alert(`Regenerating metadata for ${queued} downloaded video${queued !== 1 ? 's' : ''} in the background.`);
        } catch { /* ignore */ } finally {
            regenningIds = new Set([...regenningIds].filter(x => x !== id));
        }
    }
```

Replace with:
```ts
    async function handleRegenMetadata(id: string) {
        const ch = channels.find(c => c.id === id);
        const ok = await confirm({
            title: 'Regenerate metadata?',
            message: `Re-fetch .info.json for all downloaded videos in "${ch?.name ?? 'this channel'}" without redownloading.`,
            confirmLabel: 'Regenerate',
        });
        if (!ok) return;
        regenningIds = new Set([...regenningIds, id]);
        try {
            const { queued } = await regenChannelMetadata(id);
            toast(`Regenerating metadata for ${queued} video${queued !== 1 ? 's' : ''}`);
        } catch {
            toast('Failed to start metadata regeneration', { variant: 'error' });
        } finally {
            regenningIds = new Set([...regenningIds].filter(x => x !== id));
        }
    }
```

- [ ] **Step 4: Build to verify no TypeScript errors**

```bash
cd web && pnpm build 2>&1 | grep -E "error|Error|warn" | head -20
```

Expected: no TypeScript errors. Warnings about unused CSS are fine.

- [ ] **Step 5: Commit**

```bash
git add web/src/routes/admin/+page.svelte
git commit -m "feat: replace browser confirm/alert with ConfirmDialog and toast"
```

---

## Task 7: Build and deploy

- [ ] **Step 1: Full build**

```bash
cd /home/jamiet/code/yt-plex && mise run docker-build 2>&1 | tail -6
```

Expected: build completes, image tagged as `yt-plex:latest` (and `$REGISTRY/yt-plex:latest` if REGISTRY is set).

- [ ] **Step 2: Deploy**

```bash
mise run docker-deploy 2>&1 | tail -4
```

Expected:
```
Deploy triggered: ok
```

- [ ] **Step 3: Smoke test**

- Open the admin page
- Click "Regen metadata" on a channel — themed modal should appear
- Click Confirm — amber toast should appear bottom-right with video count
- Click "Remove" on a channel — modal should appear with "Remove channel?" title
- Click Cancel — nothing should happen

---

## Self-Review Notes

- All 6 spec requirements covered: ConfirmDialog (Tasks 1–2), Toast (Tasks 3–4), layout mount (Task 5), admin integration (Task 6)
- `confirm()` in Task 6 Step 2 shadows the store import because the old code used the browser's global `confirm`. The import is named `confirm` which replaces the global — no rename needed, this is intentional
- Toast z-index (1100) is above ConfirmDialog backdrop (1000) so a toast that fires after confirmation is never hidden behind the dialog
- No placeholders — all code is complete
