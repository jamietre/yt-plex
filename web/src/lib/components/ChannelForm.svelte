<!-- web/src/lib/components/ChannelForm.svelte -->
<script lang="ts">
    import { addChannel, updateChannel, type Channel } from '$lib/api';
    import Button from './Button.svelte';

    let { channel, basePath, onSave, onCancel }: {
        channel: Channel | null;
        basePath: string;
        onSave: (ch: Channel) => void;
        onCancel: () => void;
    } = $props();

    let name = $state(channel?.name ?? '');
    let url = $state(channel?.youtube_channel_url ?? '');
    let prefix = $state(channel?.path_prefix ?? '');
    let saving = $state(false);
    let error = $state('');

    const isEdit = $derived(channel !== null);

    const prefixHint = $derived(
        prefix.trim()
            ? `${basePath} / ${prefix.trim()} / …`
            : `${basePath} / …`
    );

    async function handleSave() {
        if (!name.trim() || !url.trim()) return;
        saving = true;
        error = '';
        try {
            const saved = isEdit
                ? await updateChannel(channel!.id, name.trim(), url.trim(), prefix.trim() || undefined)
                : await addChannel(url.trim(), name.trim(), prefix.trim() || undefined);
            onSave(saved);
        } catch (e: unknown) {
            error = e instanceof Error ? e.message : 'Save failed';
        } finally {
            saving = false;
        }
    }
</script>

<aside class="panel">
    <div class="panel-header">
        <span class="panel-title">{isEdit ? 'Edit channel' : 'Add channel'}</span>
        <button class="close-btn" onclick={onCancel} aria-label="Close">✕</button>
    </div>

    <div class="fields">
        <label class="field">
            <span class="label">Display name</span>
            <input class="input" bind:value={name} placeholder="e.g. Linus Tech Tips" />
        </label>

        <label class="field">
            <span class="label">YouTube URL</span>
            <input class="input" type="url" bind:value={url} placeholder="https://youtube.com/@Channel" />
        </label>

        <label class="field">
            <span class="label">Path prefix <span class="optional">(optional)</span></span>
            <input class="input" bind:value={prefix} placeholder="e.g. Tech" />
            <span class="hint">{prefixHint}</span>
        </label>
    </div>

    {#if error}<p class="error">{error}</p>{/if}

    <div class="actions">
        <Button variant="ghost" size="sm" onclick={onCancel}>Cancel</Button>
        <Button variant="primary" size="sm" onclick={handleSave}
                disabled={saving || !name.trim() || !url.trim()}>
            {saving ? 'Saving…' : (isEdit ? 'Save' : 'Add')}
        </Button>
    </div>
</aside>

<style>
    .panel {
        width: 260px;
        flex-shrink: 0;
        background: var(--surface-2);
        border-left: 1px solid var(--border-2);
        display: flex;
        flex-direction: column;
        gap: 0;
        align-self: stretch;
    }

    .panel-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 12px 14px 10px;
        border-bottom: 1px solid var(--border);
    }

    .panel-title {
        font-size: 12px;
        font-weight: 700;
        color: var(--text);
    }

    .close-btn {
        background: transparent;
        border: none;
        color: var(--text-3);
        font-size: 13px;
        cursor: pointer;
        padding: 0;
        line-height: 1;
    }
    .close-btn:hover { color: var(--text); }

    .fields {
        display: flex;
        flex-direction: column;
        gap: 12px;
        padding: 14px;
        flex: 1;
    }

    .field {
        display: flex;
        flex-direction: column;
        gap: 4px;
    }

    .label {
        font-size: 10px;
        font-weight: 700;
        text-transform: uppercase;
        letter-spacing: 0.6px;
        color: var(--text-3);
    }

    .optional {
        font-size: 10px;
        font-weight: 400;
        text-transform: none;
        letter-spacing: 0;
        color: var(--text-3);
    }

    .input {
        background: var(--surface-3);
        border: 1px solid var(--border-2);
        border-radius: var(--radius);
        padding: 5px 8px;
        font-size: 12px;
        font-family: var(--font-ui);
        color: var(--text);
        outline: none;
        transition: border-color 0.15s;
    }
    .input:focus { border-color: var(--amber); }
    .input::placeholder { color: var(--text-3); }

    .hint {
        font-size: 10px;
        color: var(--text-3);
        word-break: break-all;
    }

    .error {
        margin: 0 14px;
        font-size: 11px;
        color: var(--red);
    }

    .actions {
        display: flex;
        gap: 6px;
        justify-content: flex-end;
        padding: 10px 14px 14px;
        border-top: 1px solid var(--border);
    }
</style>
