<script lang="ts">
    import { onMount } from 'svelte';
    import {
        listChannels, listAllChannels, listProfileChannelIds,
        subscribeChannel, unsubscribeChannel, getProfileSession,
        type Channel,
    } from '$lib/api';

    let channels = $state<Channel[]>([]);         // subscribed channels
    let allChannels = $state<Channel[]>([]);       // all available channels
    let subscribedIds = $state(new Set<string>()); // for quick lookup
    let error = $state('');
    let showAll = $state(false);
    let profileId = $state<number | null>(null);
    let toggling = $state<string | null>(null);   // channel id being toggled

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
                // Pre-load all channels for subscription management
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
                subscribedIds = new Set(subscribedIds); // trigger reactivity
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
</script>

<main>
    {#if error}
        <p class="error">{error}</p>
    {:else}
        <div class="toolbar">
            {#if profileId && allChannels.length > 0}
                <label class="toggle-label">
                    <input
                        type="checkbox"
                        checked={showAll}
                        onchange={() => { showAll = !showAll; }}
                    />
                    Show all channels
                </label>
            {/if}
        </div>

        {#if displayChannels.length === 0 && !showAll}
            <p class="empty">
                You haven't subscribed to any channels yet.
                {#if profileId}
                    Toggle "Show all channels" above to find and subscribe to channels.
                {:else}
                    An admin can add channels from the Admin page.
                {/if}
            </p>
        {:else}
            <div class="grid">
                {#each displayChannels as channel (channel.id)}
                    <div class="card-wrap">
                        <a href="/browse/{channel.id}" class="card" class:unsubscribed={!subscribedIds.has(channel.id)}>
                            <div class="card-name">{channel.name}</div>
                            <div class="card-meta">Synced {timeAgo(channel.last_synced_at)}</div>
                        </a>
                        {#if profileId && showAll}
                            <button
                                class="sub-btn"
                                class:subscribed={subscribedIds.has(channel.id)}
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
                            </button>
                        {/if}
                    </div>
                {/each}
            </div>
        {/if}
    {/if}
</main>

<style>
    main { padding: 1rem; font-family: sans-serif; }
    .toolbar { margin-bottom: 1rem; display: flex; align-items: center; gap: 1rem; }
    .toggle-label {
        display: flex;
        align-items: center;
        gap: 0.4rem;
        font-size: 0.85rem;
        color: #888;
        cursor: pointer;
        user-select: none;
    }
    .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 0.75rem; }
    .card-wrap { display: flex; flex-direction: column; gap: 0.3rem; }
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
    .card.unsubscribed { opacity: 0.55; }
    .card-name { font-weight: 600; color: #ddd; margin-bottom: 0.3rem; }
    .card-meta { font-size: 0.8rem; color: #666; }
    .sub-btn {
        width: 100%;
        padding: 0.3rem 0;
        font-size: 0.78rem;
        border-radius: 4px;
        border: 1px solid #444;
        background: #1a1a2a;
        color: #888;
        cursor: pointer;
        transition: background 0.12s, color 0.12s, border-color 0.12s;
    }
    .sub-btn:hover:not(:disabled) { background: #22223a; color: #4af; border-color: #4af; }
    .sub-btn.subscribed { color: #4c4; border-color: #4c4; }
    .sub-btn:disabled { opacity: 0.5; cursor: default; }
    .empty { color: #888; font-style: italic; }
    .error { color: red; }
</style>
