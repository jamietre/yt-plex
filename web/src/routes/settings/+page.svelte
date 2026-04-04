<script lang="ts">
    import { onMount } from 'svelte';
    import { getSettings, updateSettings, type Settings } from '$lib/api';

    let settings = $state<Settings | null>(null);
    let error = $state('');
    let saved = $state(false);
    let saving = $state(false);

    onMount(async () => {
        try {
            settings = await getSettings();
        } catch (e: unknown) {
            const msg = e instanceof Error ? e.message : '';
            if (msg.includes('401') || msg.includes('403')) {
                window.location.href = '/login';
            } else {
                error = 'Failed to load settings — are you logged in?';
            }
        }
    });

    async function handleSave() {
        if (!settings) return;
        saving = true;
        error = '';
        saved = false;
        try {
            await updateSettings(settings);
            saved = true;
        } catch (e: unknown) {
            error = e instanceof Error ? e.message : 'Save failed';
        } finally {
            saving = false;
        }
    }
</script>

{#if settings}
<main>
    <h2>Settings</h2>
    <form onsubmit={(e) => { e.preventDefault(); handleSave(); }}>
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
            <small>Variables: {'{channel}'}, {'{date}'}, {'{title}'}, {'{ext}'}</small>
        </fieldset>
        {#if error}<p class="error">{error}</p>{/if}
        {#if saved}<p class="ok">Saved.</p>{/if}
        <button type="submit" disabled={saving}>{saving ? 'Saving…' : 'Save'}</button>
    </form>
</main>
{:else if !error}
<p>Loading…</p>
{:else}
<p class="error">{error}</p>
{/if}

<style>
    main { max-width: 560px; padding: 1rem; font-family: sans-serif; }
    fieldset { margin-bottom: 1rem; display: flex; flex-direction: column; gap: 0.5rem; border: 1px solid #555; padding: 0.75rem; }
    label { display: flex; flex-direction: column; gap: 0.2rem; font-size: 0.9rem; }
    input { padding: 0.3rem; }
    .error { color: red; }
    .ok { color: green; }
</style>
