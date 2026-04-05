<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import {
        getSettings, updateSettings, type Settings,
        listChannels, addChannel, deleteChannel, syncChannel,
        submitJob, type Channel
    } from '$lib/api';

    // Auth guard
    onMount(async () => {
        const res = await fetch('/api/auth/me');
        if (!res.ok) window.location.href = '/login';
    });

    // Settings
    let settings = $state<Settings | null>(null);
    let settingsError = $state('');
    let settingsSaved = $state(false);
    let settingsSaving = $state(false);

    // Channels
    let channels = $state<Channel[]>([]);
    let newChannelUrl = $state('');
    let newChannelName = $state('');
    let channelError = $state('');
    let addingChannel = $state(false);
    // IDs of channels currently being synced (background poll clears them)
    let syncingIds = $state(new Set<string>());
    let pollTimer: ReturnType<typeof setInterval> | null = null;

    // URL submission
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

    async function handleSyncChannel(id: string) {
        try {
            await syncChannel(id);
        } catch { /* ignore */ }
        // Snapshot last_synced_at so we can detect when it changes
        const before = channels.find(c => c.id === id)?.last_synced_at ?? null;
        syncingIds = new Set([...syncingIds, id]);
        startPolling(id, before);
    }

    function startPolling(id: string, beforeSyncedAt: string | null) {
        if (pollTimer) return; // already polling
        pollTimer = setInterval(async () => {
            try {
                const fresh = await listChannels();
                const updated = fresh.find(c => c.id === id);
                if (updated && updated.last_synced_at !== beforeSyncedAt) {
                    // Sync finished — update the list and clear syncing state
                    channels = fresh;
                    syncingIds = new Set([...syncingIds].filter(x => x !== id));
                }
                if (syncingIds.size === 0) {
                    clearInterval(pollTimer!);
                    pollTimer = null;
                }
            } catch { /* ignore */ }
        }, 3000);
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

<main>
    <h2>Admin</h2>

    <!-- Channels -->
    <section>
        <h3>Approved Channels</h3>
        <div class="add-row">
            <input bind:value={newChannelName} placeholder="Display name" />
            <input bind:value={newChannelUrl} placeholder="https://youtube.com/@ChannelName" class="url-input" />
            <button onclick={handleAddChannel} disabled={addingChannel || !newChannelUrl || !newChannelName}>
                {addingChannel ? 'Adding…' : 'Add'}
            </button>
        </div>
        {#if channelError}<p class="error">{channelError}</p>{/if}
        {#if channels.length > 0}
            <table>
                <thead><tr><th>Name</th><th>URL</th><th>Last synced</th><th></th></tr></thead>
                <tbody>
                    {#each channels as ch (ch.id)}
                        {@const syncing = syncingIds.has(ch.id)}
                        <tr class:syncing>
                            <td>{ch.name}</td>
                            <td class="url-cell"><a href={ch.youtube_channel_url} target="_blank" rel="noreferrer">{ch.youtube_channel_url}</a></td>
                            <td>
                                {#if syncing}
                                    <span class="sync-status">⟳ Syncing…</span>
                                {:else}
                                    {ch.last_synced_at ? new Date(ch.last_synced_at).toLocaleString() : 'never'}
                                {/if}
                            </td>
                            <td class="actions">
                                <button onclick={() => handleSyncChannel(ch.id)} disabled={syncing}>
                                    {syncing ? '⟳ Syncing…' : '↻ Sync'}
                                </button>
                                <button class="danger" onclick={() => handleDeleteChannel(ch.id)} disabled={syncing}>Remove</button>
                            </td>
                        </tr>
                    {/each}
                </tbody>
            </table>
        {:else}
            <p class="empty">No channels yet.</p>
        {/if}
    </section>

    <!-- URL submission -->
    <section>
        <h3>Submit URL</h3>
        <div class="add-row">
            <input type="url" bind:value={submitUrl} placeholder="https://www.youtube.com/watch?v=…" class="url-input" />
            <button onclick={handleSubmitUrl} disabled={submitting || !submitUrl}>{submitting ? 'Queuing…' : 'Queue'}</button>
        </div>
        {#if submitError}<p class="error">{submitError}</p>{/if}
        {#if submitSuccess}<p class="ok">{submitSuccess}</p>{/if}
    </section>

    <!-- Settings -->
    {#if settings}
    <section>
        <h3>Settings</h3>
        <form onsubmit={(e) => { e.preventDefault(); saveSettings(); }}>
            <fieldset>
                <legend>Plex</legend>
                <label>URL <input bind:value={settings.plex.url} /></label>
                <label>Token <input bind:value={settings.plex.token} /></label>
                <label>Library Section ID <input bind:value={settings.plex.library_section_id} /></label>
            </fieldset>
            <fieldset>
                <legend>Output</legend>
                <label>Base path <input bind:value={settings.output.base_path} /></label>
                <label>Path template <input bind:value={settings.output.path_template} /></label>
                <small>Variables: {'{channel}'}, {'{date}'}, {'{title}'}, {'{id}'}, {'{ext}'}</small>
            </fieldset>
            {#if settingsError}<p class="error">{settingsError}</p>{/if}
            {#if settingsSaved}<p class="ok">Saved.</p>{/if}
            <button type="submit" disabled={settingsSaving}>{settingsSaving ? 'Saving…' : 'Save settings'}</button>
        </form>
    </section>
    {/if}
</main>

<style>
    main { max-width: 720px; padding: 1rem; font-family: sans-serif; }
    h2 { margin-bottom: 1.5rem; color: #ddd; }
    h3 { color: #bbb; margin-bottom: 0.75rem; border-bottom: 1px solid #333; padding-bottom: 0.25rem; }
    section { margin-bottom: 2rem; }
    .add-row { display: flex; gap: 0.5rem; margin-bottom: 0.5rem; flex-wrap: wrap; }
    input { padding: 0.35rem 0.5rem; background: #1a1a2e; border: 1px solid #444; color: #ddd; border-radius: 4px; }
    .url-input { flex: 1; min-width: 200px; }
    button { padding: 0.35rem 0.75rem; background: #2a3a4a; border: 1px solid #4af; color: #4af; border-radius: 4px; cursor: pointer; font-size: 0.85rem; }
    button:disabled { opacity: 0.5; cursor: default; }
    button.danger { border-color: #f44; color: #f44; background: #2a1a1a; }
    table { width: 100%; border-collapse: collapse; font-size: 0.85rem; }
    th, td { text-align: left; padding: 0.35rem 0.5rem; border-bottom: 1px solid #333; }
    th { color: #888; }
    .url-cell { max-width: 280px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
    .url-cell a { color: #4af; text-decoration: none; }
    .actions { display: flex; gap: 0.4rem; white-space: nowrap; }
    fieldset { border: 1px solid #444; padding: 0.75rem; margin-bottom: 0.75rem; display: flex; flex-direction: column; gap: 0.5rem; }
    label { display: flex; flex-direction: column; gap: 0.2rem; font-size: 0.9rem; color: #bbb; }
    small { color: #666; font-size: 0.8rem; }
    tr.syncing td { opacity: 0.6; }
    .sync-status { color: #fa4; font-size: 0.8rem; }
    .empty { color: #666; font-style: italic; font-size: 0.85rem; }
    .error { color: #f44; font-size: 0.85rem; }
    .ok { color: #4c4; font-size: 0.85rem; }
</style>
