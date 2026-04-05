<script lang="ts">
    import { onMount } from 'svelte';
    import { page } from '$app/stores';
    import {
        getVideo, submitJobByYoutubeId, ignoreVideo, unignoreVideo,
        type Video, type VideoStatus
    } from '$lib/api';

    const channelId = $derived($page.params.channelId);
    const videoId = $derived($page.params.videoId);

    let video = $state<Video | null>(null);
    let loading = $state(true);
    let error = $state('');
    let actionWorking = $state(false);
    let actionMsg = $state('');
    let isAdmin = $state(false);

    onMount(async () => {
        const [videoResult] = await Promise.allSettled([
            getVideo(videoId),
            fetch('/api/auth/me').then(r => { isAdmin = r.ok; }),
        ]);
        if (videoResult.status === 'fulfilled') {
            video = videoResult.value;
        } else {
            error = videoResult.reason instanceof Error ? videoResult.reason.message : 'Failed to load video';
        }
        loading = false;
    });

    async function handleDownload() {
        if (!video) return;
        actionWorking = true; actionMsg = '';
        try {
            await submitJobByYoutubeId(video.youtube_id);
            video = { ...video, status: 'in_progress' as VideoStatus };
            actionMsg = 'Queued for download!';
        } catch (e: unknown) {
            actionMsg = e instanceof Error ? e.message : 'Failed';
        } finally { actionWorking = false; }
    }

    async function handleIgnore() {
        if (!video) return;
        actionWorking = true;
        try {
            await ignoreVideo(video.youtube_id);
            video = { ...video, status: 'ignored' as VideoStatus, ignored_at: new Date().toISOString() };
        } catch { /* ignore */ } finally { actionWorking = false; }
    }

    async function handleUnignore() {
        if (!video) return;
        actionWorking = true;
        try {
            await unignoreVideo(video.youtube_id);
            video = { ...video, status: 'new' as VideoStatus, ignored_at: null };
        } catch { /* ignore */ } finally { actionWorking = false; }
    }

    const statusLabel: Record<VideoStatus, string> = {
        new: 'Not downloaded',
        in_progress: 'Downloading…',
        downloaded: 'On Plex ✓',
        ignored: 'Ignored',
    };
    const statusColour: Record<VideoStatus, string> = {
        new: '#4af',
        in_progress: '#fa4',
        downloaded: '#4c4',
        ignored: '#555',
    };
</script>

<main>
    <a href="/browse/{channelId}" class="back">← Back to channel</a>

    {#if loading}
        <p class="loading">Loading…</p>
    {:else if error}
        <p class="error">{error}</p>
    {:else if video}
        <div class="detail">
            <img
                class="thumb"
                src="https://img.youtube.com/vi/{video.youtube_id}/hqdefault.jpg"
                alt={video.title}
            />
            <div class="info">
                <h1 class="title">{video.title}</h1>
                <p class="meta">
                    {#if video.published_at}
                        Added to YouTube: {new Date(video.published_at).toLocaleDateString()}
                    {:else}
                        First seen: {new Date(video.last_seen_at).toLocaleDateString()}
                    {/if}
                </p>
                <p class="status" style="color:{statusColour[video.status]}">
                    {statusLabel[video.status]}
                </p>
                {#if video.file_path}
                    <p class="file-path" title={video.file_path}>{video.file_path}</p>
                {/if}
                <div class="actions">
                    {#if video.status === 'new'}
                        <button class="btn-primary" onclick={handleDownload} disabled={actionWorking}>
                            ↓ Download
                        </button>
                        <button class="btn-secondary" onclick={handleIgnore} disabled={actionWorking}>
                            Ignore
                        </button>
                    {:else if video.status === 'in_progress'}
                        <button class="btn-disabled" disabled>Queued…</button>
                    {:else if video.status === 'downloaded'}
                        <button class="btn-disabled downloaded" disabled>On Plex ✓</button>
                    {:else if video.status === 'ignored'}
                        <button class="btn-secondary" onclick={handleUnignore} disabled={actionWorking}>
                            Unignore
                        </button>
                    {/if}
                </div>
                {#if actionMsg}<p class="action-msg">{actionMsg}</p>{/if}
                {#if isAdmin}
                    <a
                        class="yt-link"
                        href="https://www.youtube.com/watch?v={video.youtube_id}"
                        target="_blank"
                        rel="noreferrer"
                    >Watch on YouTube ↗</a>
                {/if}
            </div>
        </div>

        {#if video.description}
            <section class="description">
                <h2>Description</h2>
                <pre class="desc-text">{video.description}</pre>
            </section>
        {:else}
            <p class="loading">Loading description…</p>
        {/if}
    {/if}
</main>

<style>
    main { max-width: 860px; padding: 1rem; font-family: sans-serif; }
    .back { color: #666; text-decoration: none; font-size: 0.85rem; display: block; margin-bottom: 1rem; }
    .back:hover { color: #ccc; }
    .loading { color: #888; font-style: italic; }
    .error { color: #f44; }

    .detail { display: flex; gap: 1.5rem; margin-bottom: 1.5rem; flex-wrap: wrap; }
    .thumb { width: 320px; max-width: 100%; border-radius: 6px; flex-shrink: 0; background: #1e1e2e; }
    .info { flex: 1; min-width: 200px; }
    .title { font-size: 1.15rem; font-weight: 600; color: #ddd; margin: 0 0 0.4rem; line-height: 1.3; }
    .meta { font-size: 0.8rem; color: #666; margin: 0 0 0.5rem; }
    .status { font-weight: 600; font-size: 0.9rem; margin: 0 0 0.75rem; }
    .actions { display: flex; gap: 0.5rem; flex-wrap: wrap; margin-bottom: 0.5rem; }
    .btn-primary { background: #4af; color: #000; border: none; border-radius: 4px; padding: 0.4rem 1rem; font-size: 0.9rem; font-weight: 600; cursor: pointer; }
    .btn-secondary { background: #333; color: #aaa; border: 1px solid #555; border-radius: 4px; padding: 0.4rem 1rem; font-size: 0.9rem; cursor: pointer; }
    .btn-disabled { background: #222; color: #666; border: 1px solid #444; border-radius: 4px; padding: 0.4rem 1rem; font-size: 0.9rem; cursor: default; }
    .btn-disabled.downloaded { color: #4c4; border-color: #4c4; }
    .action-msg { font-size: 0.8rem; color: #4c4; margin: 0; }
    .file-path { font-size: 0.75rem; color: #666; font-family: monospace; word-break: break-all; margin: 0 0 0.5rem; }
    .yt-link { color: #4af; font-size: 0.8rem; text-decoration: none; }
    .yt-link:hover { text-decoration: underline; }

    .description { border-top: 1px solid #333; padding-top: 1rem; }
    .description h2 { font-size: 1rem; color: #bbb; margin: 0 0 0.5rem; }
    .desc-text {
        white-space: pre-wrap;
        word-break: break-word;
        font-family: sans-serif;
        font-size: 0.85rem;
        color: #aaa;
        line-height: 1.6;
        margin: 0;
        max-height: 400px;
        overflow-y: auto;
    }
</style>
