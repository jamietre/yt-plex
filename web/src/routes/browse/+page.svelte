<script lang="ts">
    import { onMount } from 'svelte';
    import { listChannels, type Channel } from '$lib/api';

    let channels = $state<Channel[]>([]);
    let error = $state('');

    onMount(async () => {
        try {
            channels = await listChannels();
        } catch (e: unknown) {
            error = e instanceof Error ? e.message : 'Failed to load channels';
        }
    });

    function timeAgo(isoString: string | null): string {
        if (!isoString) return 'never';
        const diff = Date.now() - new Date(isoString).getTime();
        const hours = Math.floor(diff / 3600000);
        if (hours < 1) return 'just now';
        if (hours < 24) return `${hours}h ago`;
        return `${Math.floor(hours / 24)}d ago`;
    }
</script>

<main>
    {#if error}
        <p class="error">{error}</p>
    {:else if channels.length === 0}
        <p class="empty">No channels configured yet. An admin can add channels from the Admin page.</p>
    {:else}
        <div class="grid">
            {#each channels as channel (channel.id)}
                <a href="/browse/{channel.id}" class="card">
                    <div class="card-name">{channel.name}</div>
                    <div class="card-meta">Synced {timeAgo(channel.last_synced_at)}</div>
                </a>
            {/each}
        </div>
    {/if}
</main>

<style>
    main { padding: 1rem; font-family: sans-serif; }
    .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 0.75rem; }
    .card {
        display: block;
        background: #1e1e2e;
        border: 1px solid #2a2a3a;
        border-radius: 8px;
        padding: 1rem;
        text-decoration: none;
        color: inherit;
        transition: border-color 0.15s;
    }
    .card:hover { border-color: #4af; }
    .card-name { font-weight: 600; color: #ddd; margin-bottom: 0.3rem; }
    .card-meta { font-size: 0.8rem; color: #666; }
    .empty { color: #888; font-style: italic; }
    .error { color: red; }
</style>
