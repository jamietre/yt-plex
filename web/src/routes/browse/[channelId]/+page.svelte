<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { page } from '$app/stores';
    import {
        listChannels, listVideos, ignoreVideo, unignoreVideo,
        submitJobByYoutubeId, syncChannel, type Channel, type Video, type VideoStatus
    } from '$lib/api';
    import { createWsStore } from '$lib/ws';

    const channelId = $derived($page.params.channelId);

    let channel = $state<Channel | null>(null);
    let videos = $state<Video[]>([]);
    let filter = $state<'new' | 'downloaded' | 'all'>('new');
    let showIgnored = $state(false);
    let error = $state('');
    let syncing = $state(false);

    const ws = createWsStore();
    let unsubWs: (() => void) | undefined;

    async function loadVideos() {
        try {
            videos = await listVideos(channelId, filter, showIgnored);
        } catch (e: unknown) {
            error = e instanceof Error ? e.message : 'Failed to load videos';
        }
    }

    onMount(async () => {
        try {
            const channels = await listChannels();
            channel = channels.find(c => c.id === channelId) ?? null;
        } catch { /* ignore */ }
        await loadVideos();
        ws.connect();
        unsubWs = ws.subscribe(() => {});
    });

    onDestroy(() => { ws.disconnect(); unsubWs?.(); });

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

    async function handleDownload(video: Video) {
        try {
            await submitJobByYoutubeId(video.youtube_id);
            // Optimistically update to in_progress
            videos = videos.map(v =>
                v.youtube_id === video.youtube_id ? { ...v, status: 'in_progress' as VideoStatus } : v
            );
        } catch (e: unknown) {
            alert(e instanceof Error ? e.message : 'Failed to queue download');
        }
    }

    async function handleIgnore(video: Video) {
        try {
            await ignoreVideo(video.youtube_id);
            videos = videos.map(v =>
                v.youtube_id === video.youtube_id ? { ...v, status: 'ignored' as VideoStatus, ignored_at: new Date().toISOString() } : v
            );
        } catch { /* ignore */ }
    }

    async function handleUnignore(video: Video) {
        try {
            await unignoreVideo(video.youtube_id);
            videos = videos.map(v =>
                v.youtube_id === video.youtube_id ? { ...v, status: 'new' as VideoStatus, ignored_at: null } : v
            );
        } catch { /* ignore */ }
    }

    async function handleSync() {
        syncing = true;
        try {
            await syncChannel(channelId);
            // Reload after a brief delay to let sync start
            setTimeout(loadVideos, 2000);
        } catch { /* ignore */ } finally {
            syncing = false;
        }
    }

    $effect(() => {
        // Reload when filter or showIgnored changes
        void filter;
        void showIgnored;
        loadVideos();
    });

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

    {#if error}<p class="error">{error}</p>{/if}

    <div class="grid">
        {#each videos as video (video.youtube_id)}
            <div class="card">
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
                <div class="card-body">
                    <div class="title" title={video.title}>{video.title}</div>
                    <div class="actions">
                        {#if video.status === 'new'}
                            <button class="btn-download" onclick={() => handleDownload(video)}>Download</button>
                            <button class="btn-ignore" onclick={() => handleIgnore(video)}>Ignore</button>
                        {:else if video.status === 'in_progress'}
                            <button class="btn-status" disabled>Queued…</button>
                        {:else if video.status === 'downloaded'}
                            <button class="btn-status downloaded" disabled>On Plex ✓</button>
                        {:else if video.status === 'ignored'}
                            <button class="btn-ignore" onclick={() => handleUnignore(video)}>Unignore</button>
                        {/if}
                    </div>
                </div>
            </div>
        {/each}
        {#if videos.length === 0 && !error}
            <p class="empty">No videos match this filter.</p>
        {/if}
    </div>
</main>

<style>
    main { padding: 1rem; font-family: sans-serif; }
    .header { display: flex; align-items: center; gap: 0.75rem; margin-bottom: 0.75rem; }
    .back { color: #666; text-decoration: none; font-size: 0.85rem; }
    .back:hover { color: #ccc; }
    .channel-name { font-weight: 600; color: #ddd; }
    .refresh { margin-left: auto; background: none; border: 1px solid #444; color: #888; padding: 0.2rem 0.6rem; border-radius: 4px; cursor: pointer; font-size: 0.8rem; }
    .refresh:hover:not(:disabled) { border-color: #4af; color: #4af; }

    .filters { display: flex; align-items: center; gap: 0.4rem; margin-bottom: 0.75rem; flex-wrap: wrap; }
    .label { font-size: 0.8rem; color: #666; }
    .pill { background: #222; color: #888; border: 1px solid #333; border-radius: 12px; padding: 0.2rem 0.7rem; font-size: 0.8rem; cursor: pointer; }
    .pill.active { background: #4af; color: #000; border-color: #4af; font-weight: 600; }
    .toggle { margin-left: auto; display: flex; align-items: center; gap: 0.3rem; font-size: 0.8rem; color: #666; cursor: pointer; }

    .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(180px, 1fr)); gap: 0.6rem; }
    .card { background: #1e1e2e; border: 1px solid #2a2a3a; border-radius: 6px; overflow: hidden; }
    .thumb { position: relative; }
    .thumb img { width: 100%; aspect-ratio: 16/9; object-fit: cover; display: block; background: #2a2a4a; }
    .badge { position: absolute; top: 4px; right: 4px; font-size: 0.6rem; font-weight: 700; padding: 2px 5px; border-radius: 3px; color: #000; }
    .card-body { padding: 0.4rem 0.5rem; }
    .title { font-size: 0.75rem; color: #ddd; line-height: 1.3; margin-bottom: 0.4rem; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
    .actions { display: flex; gap: 0.3rem; }
    .btn-download { flex: 1; background: #4af; color: #000; border: none; border-radius: 3px; padding: 0.2rem; font-size: 0.7rem; font-weight: 600; cursor: pointer; }
    .btn-ignore { background: #333; color: #777; border: none; border-radius: 3px; padding: 0.2rem 0.5rem; font-size: 0.7rem; cursor: pointer; }
    .btn-status { flex: 1; background: #222; color: #666; border: 1px solid #444; border-radius: 3px; padding: 0.2rem; font-size: 0.7rem; cursor: default; }
    .btn-status.downloaded { color: #4c4; border-color: #4c4; }
    .empty { color: #888; font-style: italic; grid-column: 1/-1; }
    .error { color: red; }
</style>
