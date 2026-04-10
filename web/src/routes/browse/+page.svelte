<script lang="ts">
    import { onMount } from 'svelte';
    import {
        listAllChannels,
        listProfileChannelIds,
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
            const session = await getProfileSession();
            profileId = session?.id ?? null;

            if (profileId) {
                const [all, ids] = await Promise.all([
                    listAllChannels(),
                    listProfileChannelIds(profileId),
                ]);
                allChannels = all;
                subscribedIds = new Set(ids);
                channels = all.filter(c => subscribedIds.has(c.id));
            } else {
                allChannels = await listAllChannels();
                channels = allChannels;
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
