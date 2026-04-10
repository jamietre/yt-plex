<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { page } from '$app/stores';
    import { navSearch } from '$lib/navSearch';
    import {
        listChannels, listVideos, ignoreVideo, unignoreVideo,
        submitJobByYoutubeId, syncChannel,
        type Channel, type Video, type VideoStatus
    } from '$lib/api';
    import { wsMessages } from '$lib/ws';
    import { toast } from '$lib/toast';
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

    let searchTimer: ReturnType<typeof setTimeout> | null = null;
    let sentinel: HTMLDivElement | undefined;
    let observer: IntersectionObserver | null = null;

    async function loadPage(reset: boolean) {
        if (loading) return;
        loading = true;
        const currentOffset = reset ? 0 : offset;
        try {
            const result = await listVideos(channelId ?? '', filter, showIgnored, ftsSearch, LIMIT, currentOffset);
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
        navSearch.set({ value: search, placeholder: 'Search titles…', onInput: handleSearchInput });
        try {
            const channels = await listChannels();
            channel = channels.find(c => c.id === channelId) ?? null;
        } catch { /* ignore */ }
        await loadPage(true);

        observer = new IntersectionObserver((entries) => {
            if (entries[0].isIntersecting && hasMore && !loading) loadPage(false);
        }, { rootMargin: '200px' });
        if (sentinel) observer.observe(sentinel);
    });

    onDestroy(() => {
        navSearch.set(null);
        observer?.disconnect();
        if (searchTimer) clearTimeout(searchTimer);
    });

    $effect(() => {
        const msg = $wsMessages;
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

    // Keep nav search input value in sync
    $effect(() => {
        navSearch.update(s => s ? { ...s, value: search } : s);
    });

    // Words with ≥ 3 chars go to the server FTS index.
    // Words with < 3 chars (e.g. "3", "S4") are post-filtered on the client
    // because FTS5 doesn't index tokens shorter than 3 characters.
    const ftsSearch = $derived(
        search.split(/\s+/).filter(w => w.length >= 3).join(' ') || ''
    );
    const shortTerms = $derived(
        search.split(/\s+/).filter(w => w.length > 0 && w.length < 3).map(w => w.toLowerCase())
    );
    const displayVideos = $derived(
        shortTerms.length === 0
            ? videos
            : videos.filter(v => shortTerms.every(t => v.title.toLowerCase().includes(t)))
    );

    function handleSearchInput(value: string) {
        search = value;
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
            toast(e instanceof Error ? e.message : 'Failed to queue download', { variant: 'error' });
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
        try { await syncChannel(channelId ?? ''); setTimeout(resetAndLoad, 2000); }
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

    function isRecentlyPublished(publishedAt: string | null): boolean {
        if (!publishedAt) return false;
        return Date.now() - new Date(publishedAt).getTime() < 7 * 24 * 60 * 60 * 1000;
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

    <PageHeader title={channel?.name ?? channelId ?? ''}>
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
        {#each displayVideos as video (video.youtube_id)}
            {@const isSelected = selected.has(video.youtube_id)}
            {@const isNew = isRecentlyPublished(video.published_at)}
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
                        {#if isNew}<span class="new-badge">NEW</span>{/if}
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
    .new-badge {
        position: absolute;
        bottom: 5px;
        left: 5px;
        background: var(--amber);
        color: #000;
        font-size: 8px;
        font-weight: 800;
        font-family: var(--font-ui);
        letter-spacing: 0.8px;
        padding: 2px 5px;
        border-radius: 3px;
        line-height: 1;
    }

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
