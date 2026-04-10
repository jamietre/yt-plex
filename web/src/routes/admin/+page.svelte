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
        listPlexLibraries, type PlexLibrary,
        listChannels, addChannel, updateChannel, deleteChannel, syncChannel,
        rescanFilesystem, regenChannelMetadata,
        submitJob, listProfiles, createProfile, deleteProfile,
        type Channel, type Profile,
    } from '$lib/api';
    import ChannelForm from '$lib/components/ChannelForm.svelte';
    import { showConfirm } from '$lib/confirm';
    import { toast } from '$lib/toast';

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
    let plexLibraries = $state<PlexLibrary[]>([]);
    let plexLibrariesError = $state('');
    let fetchingLibraries = $state(false);
    async function fetchPlexLibraries() {
        fetchingLibraries = true;
        plexLibrariesError = '';
        try {
            plexLibraries = await listPlexLibraries();
            // Drop any saved IDs that no longer exist in Plex
            if (settings) {
                const validIds = new Set(plexLibraries.map(l => l.id));
                const filtered = selectedLibraryIds.filter(id => validIds.has(id));
                settings = { ...settings, plex: { ...settings.plex, library_section_id: filtered.join(', ') } };
            }
        } catch (e: unknown) {
            plexLibrariesError = e instanceof Error ? e.message : 'Failed to fetch libraries';
        } finally {
            fetchingLibraries = false;
        }
    }

    function toggleLibrary(id: string) {
        if (!settings) return;
        const current = settings.plex.library_section_id
            .split(',').map(s => s.trim()).filter(Boolean);
        const next = current.includes(id)
            ? current.filter(s => s !== id)
            : [...current, id];
        settings = { ...settings, plex: { ...settings.plex, library_section_id: next.join(', ') } };
    }

    const selectedLibraryIds = $derived(
        settings?.plex.library_section_id.split(',').map(s => s.trim()).filter(Boolean) ?? []
    );

    // ── Channels ─────────────────────────────────────────────────────────────
    let channels = $state<Channel[]>([]);
    let formOpen = $state(false);
    let editingChannel = $state<Channel | null>(null);
    let syncingIds = $state(new Set<string>());
    let regenningIds = $state(new Set<string>());
    let pollTimer: ReturnType<typeof setInterval> | null = null;
    let rescanning = $state(false);
    let rescanMsg = $state('');

    // ── URL submission ────────────────────────────────────────────────────────
    let submitUrl = $state('');
    let submitError = $state('');
    let submitSuccess = $state('');
    let submitting = $state(false);

    onMount(async () => {
        try {
            const s = await getSettings();
            settings = { plex: s.plex, output: s.output, download: s.download ?? { extra_args: [] } };
        } catch { settingsError = 'Failed to load settings'; }
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

    function openAdd() {
        editingChannel = null;
        formOpen = true;
    }

    function openEdit(ch: Channel) {
        editingChannel = ch;
        formOpen = true;
    }

    function closeForm() {
        formOpen = false;
        editingChannel = null;
    }

    function handleFormSave(saved: Channel) {
        if (editingChannel) {
            channels = channels.map(c => c.id === saved.id ? saved : c);
        } else {
            channels = [...channels, saved];
        }
        closeForm();
    }

    async function handleDeleteChannel(id: string) {
        const ok = await showConfirm({
            title: 'Remove channel?',
            message: 'This removes the channel and all its video metadata. Downloaded files are not deleted.',
            confirmLabel: 'Remove',
        });
        if (!ok) return;
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

    async function handleRegenMetadata(id: string) {
        const ch = channels.find(c => c.id === id);
        const ok = await showConfirm({
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
        const ok = await showConfirm({
            title: `Remove profile?`,
            message: `Remove "${name}"? This will delete their ignore list and channel subscriptions.`,
            confirmLabel: 'Remove',
        });
        if (!ok) return;
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

            <div class="channels-toolbar">
                <Button variant="primary" size="sm" onclick={openAdd}>+ Add channel</Button>
            </div>

            <div class="channels-layout">
                <div class="channels-table-wrap">
                    {#if channels.length > 0}
                        <table class="data-table">
                            <thead>
                                <tr><th>Name</th><th>Prefix</th><th>Channel ID</th><th>Last synced</th><th></th></tr>
                            </thead>
                            <tbody>
                                {#each channels as ch (ch.id)}
                                    {@const syncing = syncingIds.has(ch.id)}
                                    <tr class:row-dim={syncing} class:row-editing={editingChannel?.id === ch.id}>
                                        <td class="td-primary">{ch.name}</td>
                                        <td class="td-mono">{ch.path_prefix ?? '—'}</td>
                                        <td class="td-mono">{ch.youtube_channel_id ?? '—'}</td>
                                        <td>
                                            {#if syncing}
                                                <span class="sync-status">⟳ Syncing…</span>
                                            {:else}
                                                {ch.last_synced_at ? new Date(ch.last_synced_at).toLocaleString() : 'never'}
                                            {/if}
                                        </td>
                                        <td class="td-actions">
                                            <Button variant="secondary" size="sm" onclick={() => openEdit(ch)}>
                                                Edit
                                            </Button>
                                            <Button variant="secondary" size="sm" onclick={() => handleSyncChannel(ch.id)} disabled={syncing}>
                                                ↻ Sync
                                            </Button>
                                            <Button variant="secondary" size="sm" onclick={() => handleRegenMetadata(ch.id)} disabled={regenningIds.has(ch.id)}>
                                                ⟳ Regen metadata
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
                            {rescanning ? 'Rescanning…' : '↺ Rescan filesystem'}
                        </Button>
                        {#if rescanMsg}<span class="rescan-msg">{rescanMsg}</span>{/if}
                    </div>
                </div>

                {#if formOpen}
                    <ChannelForm
                        channel={editingChannel}
                        basePath={settings?.output.base_path ?? ''}
                        onSave={handleFormSave}
                        onCancel={closeForm}
                    />
                {/if}
            </div>

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
                        <div class="lib-row">
                            <span class="lib-label">Libraries</span>
                            <button type="button" class="lib-fetch-btn" onclick={fetchPlexLibraries} disabled={fetchingLibraries}>
                                {fetchingLibraries ? 'Fetching…' : plexLibraries.length ? 'Refresh' : 'Fetch from Plex'}
                            </button>
                        </div>
                        {#if plexLibrariesError}
                            <p class="msg-error">{plexLibrariesError}</p>
                        {:else if plexLibraries.length > 0}
                            <div class="lib-checklist">
                                {#each plexLibraries as lib}
                                    <label class="lib-check">
                                        <input
                                            type="checkbox"
                                            checked={selectedLibraryIds.includes(lib.id)}
                                            onchange={() => toggleLibrary(lib.id)}
                                        />
                                        {lib.title} <span class="lib-id">(id# {lib.id})</span>
                                    </label>
                                {/each}
                            </div>
                        {:else if selectedLibraryIds.length > 0}
                            <p class="lib-hint">IDs: {settings.plex.library_section_id}</p>
                        {/if}
                    </fieldset>
                    <fieldset>
                        <legend>Download</legend>
                        <label>
                            Extra yt-dlp args
                            <textarea
                                class="args-textarea"
                                rows="4"
                                placeholder={"--cookies /path/to/cookies.txt\n--rate-limit 2M"}
                                value={settings.download.extra_args.join('\n')}
                                oninput={(e) => {
                                    if (!settings) return;
                                    const lines = (e.target as HTMLTextAreaElement).value
                                        .split('\n').map(s => s.trim()).filter(Boolean);
                                    settings = { plex: settings.plex, output: settings.output, download: { extra_args: lines } };
                                }}
                            ></textarea>
                            <small>One argument per line. Inserted before the URL when running yt-dlp.</small>
                        </label>
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

    /* ── Channels layout ─────────────────────────────────────────────────── */
    .channels-toolbar {
        display: flex;
        justify-content: flex-end;
        margin-bottom: 12px;
    }

    .channels-layout {
        display: flex;
        align-items: flex-start;
        gap: 0;
    }

    .channels-table-wrap {
        flex: 1;
        min-width: 0;
    }

    .row-editing td { background: rgba(232, 160, 32, 0.05); }

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
    .rescan-msg { font-size: 11px; color: var(--text-3); }

    /* ── Messages ─────────────────────────────────────────────────────────── */
    .msg-error { color: var(--red);   font-size: 12px; margin: 6px 0; }
    .msg-ok    { color: var(--green); font-size: 12px; margin: 6px 0; }

    /* ── Plex library picker ─────────────────────────────────────────────── */
    .lib-row { display: flex; align-items: center; justify-content: space-between; }
    .lib-label { font-size: 12px; color: var(--text-2); }
    .lib-fetch-btn {
        background: none; border: none; color: var(--amber);
        font-size: 11px; font-family: var(--font-ui); cursor: pointer; padding: 0;
    }
    .lib-fetch-btn:hover { text-decoration: underline; }
    .lib-fetch-btn:disabled { opacity: 0.5; cursor: default; text-decoration: none; }
    .lib-checklist { display: flex; flex-direction: column; margin-top: 4px; }
    .lib-check {
        display: flex; flex-direction: row; align-items: center; gap: 5px;
        font-size: 11px; color: var(--text-2); cursor: pointer;
        padding: 1px 0; line-height: 1.4; white-space: nowrap;
    }
    .lib-check input { accent-color: var(--amber); width: 11px; height: 11px; flex-shrink: 0; }
    .lib-check:has(input:checked) { color: var(--text); }
    .lib-id { color: var(--text-3); }
    .lib-hint { font-size: 11px; color: var(--text-3); margin: 4px 0; }
    .args-textarea {
        font-family: monospace;
        font-size: 11px;
        background: var(--surface-2);
        border: 1px solid var(--border);
        color: var(--text);
        border-radius: var(--radius);
        padding: 6px 8px;
        resize: vertical;
        outline: none;
        width: 100%;
        box-sizing: border-box;
        line-height: 1.6;
        transition: border-color 0.15s;
    }
    .args-textarea:focus { border-color: var(--amber); }
    .args-textarea::placeholder { color: var(--text-3); }

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
