<script lang="ts">
    import { onMount } from 'svelte';
    import { page } from '$app/stores';
    import {
        getVideo, submitJobByYoutubeId, ignoreVideo, unignoreVideo,
        type Video, type VideoStatus
    } from '$lib/api';
    import Badge from '$lib/components/Badge.svelte';
    import Button from '$lib/components/Button.svelte';

    const channelId = $derived($page.params.channelId);
    const videoId   = $derived($page.params.videoId ?? '');

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
</script>

<div class="page">
    <a href="/browse/{channelId}" class="back-link">← Back to channel</a>

    {#if loading}
        <p class="hint">Loading…</p>
    {:else if error}
        <p class="msg-error">{error}</p>
    {:else if video}
        <div class="detail">
            <img class="thumb" src="/api/thumbnails/{video.youtube_id}" alt={video.title} />
            <div class="info">
                <h1 class="title">{video.title}</h1>
                <p class="meta">
                    {#if video.published_at}
                        Added to YouTube: {new Date(video.published_at).toLocaleDateString()}
                    {:else}
                        First seen: {new Date(video.last_seen_at).toLocaleDateString()}
                    {/if}
                </p>
                <div class="status-row">
                    <Badge status={video.status} />
                </div>
                {#if video.file_path}
                    <p class="file-path" title={video.file_path}>{video.file_path}</p>
                {/if}
                <div class="actions">
                    {#if video.status === 'new'}
                        <Button variant="primary" onclick={handleDownload} disabled={actionWorking}>↓ Download</Button>
                        <Button variant="secondary" onclick={handleIgnore} disabled={actionWorking}>Ignore</Button>
                    {:else if video.status === 'in_progress'}
                        <Button variant="secondary" disabled>Queued…</Button>
                    {:else if video.status === 'downloaded'}
                        <Button variant="secondary" disabled>On Plex ✓</Button>
                    {:else if video.status === 'ignored'}
                        <Button variant="secondary" onclick={handleUnignore} disabled={actionWorking}>Unignore</Button>
                    {/if}
                </div>
                {#if actionMsg}<p class="action-msg">{actionMsg}</p>{/if}
                {#if isAdmin}
                    <a class="yt-link" href="https://www.youtube.com/watch?v={video.youtube_id}" target="_blank" rel="noreferrer">
                        Watch on YouTube ↗
                    </a>
                {/if}
            </div>
        </div>

        {#if video.description}
            <section class="description">
                <h2 class="desc-heading">Description</h2>
                <pre class="desc-text">{video.description}</pre>
            </section>
        {/if}
    {/if}
</div>

<style>
    .page { max-width: 860px; padding: 20px 24px; }

    .back-link {
        display: inline-block;
        font-size: 12px;
        color: var(--text-3);
        text-decoration: none;
        margin-bottom: 16px;
        transition: color 0.15s;
    }
    .back-link:hover { color: var(--amber); }

    .hint     { color: var(--text-3); font-style: italic; font-size: 13px; }
    .msg-error{ color: var(--red);    font-size: 13px; }

    .detail { display: flex; gap: 24px; margin-bottom: 24px; flex-wrap: wrap; }
    .thumb  {
        width: 320px;
        max-width: 100%;
        border-radius: var(--radius);
        flex-shrink: 0;
        background: var(--surface-3);
        display: block;
    }
    .info { flex: 1; min-width: 200px; }

    .title {
        font-family: var(--font-display);
        font-size: 20px;
        font-weight: 700;
        color: var(--text);
        margin: 0 0 6px;
        line-height: 1.3;
    }
    .meta        { font-size: 11px; color: var(--text-3); margin: 0 0 10px; }
    .status-row  { margin-bottom: 10px; }
    .file-path   { font-size: 11px; color: var(--text-3); font-family: monospace; word-break: break-all; margin: 0 0 10px; }
    .actions     { display: flex; gap: 8px; flex-wrap: wrap; margin-bottom: 8px; }
    .action-msg  { font-size: 12px; color: var(--green); margin: 0; }
    .yt-link     { font-size: 12px; color: var(--amber); text-decoration: none; }
    .yt-link:hover { text-decoration: underline; }

    .description   { border-top: 1px solid var(--border); padding-top: 16px; }
    .desc-heading  { font-size: 14px; font-weight: 600; color: var(--text-2); margin: 0 0 8px; }
    .desc-text {
        white-space: pre-wrap;
        word-break: break-word;
        font-family: var(--font-ui);
        font-size: 13px;
        color: var(--text-2);
        line-height: 1.65;
        margin: 0;
        max-height: 400px;
        overflow-y: auto;
    }
</style>
