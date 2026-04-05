<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { page } from '$app/stores';
    import {
        listChannels, listVideos, ignoreVideo, unignoreVideo,
        submitJobByYoutubeId, syncChannel,
        type Channel, type Video, type VideoStatus
    } from '$lib/api';
    import { createWsStore } from '$lib/ws';

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

    // Multi-select
    let selected = $state(new Set<string>());
    let bulkWorking = $state(false);

    const ws = createWsStore();
    let unsubWs: (() => void) | undefined;

    // Debounce search: only reload after 300ms idle
    let searchTimer: ReturnType<typeof setTimeout> | null = null;

    // Sentinel element for IntersectionObserver
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

    function resetAndLoad() {
        offset = 0;
        hasMore = false;
        loadPage(true);
    }

    onMount(async () => {
        try {
            const channels = await listChannels();
            channel = channels.find(c => c.id === channelId) ?? null;
        } catch { /* ignore */ }
        await loadPage(true);
        ws.connect();
        unsubWs = ws.subscribe(() => {});

        // Set up IntersectionObserver on the sentinel
        observer = new IntersectionObserver((entries) => {
            if (entries[0].isIntersecting && hasMore && !loading) {
                loadPage(false);
            }
        }, { rootMargin: '200px' });
        if (sentinel) observer.observe(sentinel);
    });

    onDestroy(() => {
        ws.disconnect();
        unsubWs?.();
        observer?.disconnect();
        if (searchTimer) clearTimeout(searchTimer);
    });

    // Re-observe sentinel when it mounts (after first render)
    $effect(() => {
        if (sentinel && observer) observer.observe(sentinel);
    });

    // Apply real-time WS updates to video status
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

    // Reload when filter or showIgnored changes
    $effect(() => {
        void filter;
        void showIgnored;
        resetAndLoad();
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
        try {
            await syncChannel(channelId);
            setTimeout(resetAndLoad, 2000);
        } catch { /* ignore */ } finally {
            syncing = false;
        }
    }

    function toggleSelect(youtubeId: string) {
        const next = new Set(selected);
        if (next.has(youtubeId)) next.delete(youtubeId);
        else next.add(youtubeId);
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
        for (const id of selected) {
            await handleIgnore(id).catch(() => {});
        }
        selected = new Set();
        bulkWorking = false;
    }

    const statusLabel: Record<VideoStatus, string> = {
        new: 'NEW',
        in_progress: '↓',
        downloaded: '✓ ON PLEX',
        ignored: 'IGNORED',
    };
    const statusColour: Record<VideoStatus, string> = {
        new: '#4af',
        in_progress: '#fa4',
        downloaded: '#4c4',
        ignored: '#555',
    };
</script>

<main>
    <div class="header">
        <a href="/browse" class="back">← Channels</a>
        <span class="channel-name">{channel?.name ?? channelId}</span>
        <button class="refresh" onclick={handleSync} disabled={syncing}>
            {syncing ? 'Syncing…' : '↻ Refresh'}
        </button>
    </div>

    <div class="toolbar">
        <div class="filters">
            <span class="label">Show:</span>
            {#each (['new', 'downloaded', 'all'] as const) as f}
                <button
                    class="pill"
                    class:active={filter === f}
                    onclick={() => { filter = f; }}
                >{f}</button>
            {/each}
            <label class="toggle">
                <input type="checkbox" bind:checked={showIgnored} />
                Show ignored
            </label>
        </div>
        <input
            class="search"
            type="search"
            placeholder="Search titles…"
            value={search}
            oninput={handleSearchInput}
        />
    </div>

    {#if selected.size > 0}
        <div class="bulk-bar">
            <span>{selected.size} selected</span>
            <button onclick={bulkDownload} disabled={bulkWorking}>↓ Download all</button>
            <button onclick={bulkIgnore} disabled={bulkWorking}>✕ Ignore all</button>
            <button class="clear" onclick={() => selected = new Set()}>Clear</button>
        </div>
    {/if}

    {#if error}<p class="error">{error}</p>{/if}

    <div class="grid">
        {#each videos as video (video.youtube_id)}
            {@const isSelected = selected.has(video.youtube_id)}
            <div class="card" class:card-selected={isSelected}>
                <label class="check-wrap" title="Select">
                    <input
                        type="checkbox"
                        class="card-check"
                        checked={isSelected}
                        onchange={() => toggleSelect(video.youtube_id)}
                    />
                </label>
                <a href="/browse/{channelId}/{video.youtube_id}" class="thumb-link">
                    <div class="thumb">
                        <img
                            src="https://img.youtube.com/vi/{video.youtube_id}/mqdefault.jpg"
                            alt={video.title}
                            loading="lazy"
                        />
                        <span class="badge" style="background:{statusColour[video.status]}">
                            {statusLabel[video.status]}
                        </span>
                    </div>
                </a>
                <div class="card-body">
                    <a href="/browse/{channelId}/{video.youtube_id}" class="title" title={video.title}>
                        {video.title}
                    </a>
                    <div class="actions">
                        {#if video.status === 'new'}
                            <button class="btn-download" onclick={() => handleDownload(video.youtube_id)}>Download</button>
                            <button class="btn-ignore" onclick={() => handleIgnore(video.youtube_id)}>✕</button>
                        {:else if video.status === 'in_progress'}
                            <button class="btn-status" disabled>Queued…</button>
                        {:else if video.status === 'downloaded'}
                            <button class="btn-status downloaded" disabled>On Plex ✓</button>
                        {:else if video.status === 'ignored'}
                            <button class="btn-ignore" onclick={() => handleUnignore(video.youtube_id)}>Unignore</button>
                        {/if}
                    </div>
                </div>
            </div>
        {/each}
        {#if videos.length === 0 && !loading && !error}
            <p class="empty">No videos match this filter.</p>
        {/if}
    </div>

    {#if loading}
        <p class="loading-msg">Loading…</p>
    {/if}

    <!-- Sentinel: observed by IntersectionObserver to trigger next page -->
    <div bind:this={sentinel} class="sentinel"></div>
</main>

<style>
    main { padding: 1rem; font-family: sans-serif; }
    .header { display: flex; align-items: center; gap: 0.75rem; margin-bottom: 0.75rem; }
    .back { color: #666; text-decoration: none; font-size: 0.85rem; }
    .back:hover { color: #ccc; }
    .channel-name { font-weight: 600; color: #ddd; }
    .refresh { margin-left: auto; background: none; border: 1px solid #444; color: #888; padding: 0.2rem 0.6rem; border-radius: 4px; cursor: pointer; font-size: 0.8rem; }
    .refresh:hover:not(:disabled) { border-color: #4af; color: #4af; }

    .toolbar { display: flex; align-items: center; gap: 0.75rem; margin-bottom: 0.75rem; flex-wrap: wrap; }
    .filters { display: flex; align-items: center; gap: 0.4rem; flex-wrap: wrap; }
    .label { font-size: 0.8rem; color: #666; }
    .pill { background: #222; color: #888; border: 1px solid #333; border-radius: 12px; padding: 0.2rem 0.7rem; font-size: 0.8rem; cursor: pointer; }
    .pill.active { background: #4af; color: #000; border-color: #4af; font-weight: 600; }
    .toggle { display: flex; align-items: center; gap: 0.3rem; font-size: 0.8rem; color: #666; cursor: pointer; }
    .search { margin-left: auto; padding: 0.3rem 0.6rem; background: #1a1a2e; border: 1px solid #444; color: #ddd; border-radius: 16px; font-size: 0.85rem; min-width: 180px; }
    .search:focus { outline: none; border-color: #4af; }

    .bulk-bar {
        display: flex; align-items: center; gap: 0.5rem;
        background: #1e2a3a; border: 1px solid #4af; border-radius: 6px;
        padding: 0.4rem 0.75rem; margin-bottom: 0.75rem; font-size: 0.85rem; color: #ddd;
    }
    .bulk-bar span { margin-right: auto; }
    .bulk-bar button { background: #2a4a6a; border: 1px solid #4af; color: #4af; border-radius: 4px; padding: 0.2rem 0.6rem; cursor: pointer; font-size: 0.8rem; }
    .bulk-bar button:disabled { opacity: 0.5; cursor: default; }
    .bulk-bar button.clear { border-color: #888; color: #888; background: none; }

    .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(180px, 1fr)); gap: 0.6rem; }
    .card { background: #1e1e2e; border: 1px solid #2a2a3a; border-radius: 6px; overflow: hidden; position: relative; }
    .card-selected { border-color: #4af; box-shadow: 0 0 0 1px #4af; }
    .check-wrap { position: absolute; top: 4px; left: 4px; z-index: 2; cursor: pointer; }
    .card-check { width: 16px; height: 16px; accent-color: #4af; cursor: pointer; }
    .thumb-link { display: block; text-decoration: none; }
    .thumb { position: relative; }
    .thumb img { width: 100%; aspect-ratio: 16/9; object-fit: cover; display: block; background: #2a2a4a; }
    .badge { position: absolute; top: 4px; right: 4px; font-size: 0.6rem; font-weight: 700; padding: 2px 5px; border-radius: 3px; color: #000; }
    .card-body { padding: 0.4rem 0.5rem; }
    .title { font-size: 0.75rem; color: #ddd; line-height: 1.3; margin-bottom: 0.4rem; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; text-decoration: none; }
    .title:hover { color: #4af; }
    .actions { display: flex; gap: 0.3rem; }
    .btn-download { flex: 1; background: #4af; color: #000; border: none; border-radius: 3px; padding: 0.2rem; font-size: 0.7rem; font-weight: 600; cursor: pointer; }
    .btn-ignore { background: #333; color: #777; border: none; border-radius: 3px; padding: 0.2rem 0.5rem; font-size: 0.7rem; cursor: pointer; }
    .btn-status { flex: 1; background: #222; color: #666; border: 1px solid #444; border-radius: 3px; padding: 0.2rem; font-size: 0.7rem; cursor: default; }
    .btn-status.downloaded { color: #4c4; border-color: #4c4; }
    .empty, .loading-msg { color: #888; font-style: italic; grid-column: 1/-1; text-align: center; padding: 2rem; }
    .error { color: red; }
    .sentinel { height: 1px; }
</style>
