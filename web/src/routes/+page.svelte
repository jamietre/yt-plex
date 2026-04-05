<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { listJobs, submitJob, type Job } from '$lib/api';
    import { createWsStore } from '$lib/ws';

    let jobs = $state<Job[]>([]);
    let url = $state('');
    let submitError = $state('');
    let submitting = $state(false);

    const ws = createWsStore();

    onMount(async () => {
        try {
            jobs = await listJobs();
        } catch {
            // ignore — might just be no jobs yet
        }
        ws.connect();
    });

    onDestroy(() => ws.disconnect());

    // Apply live updates from WebSocket
    $effect(() => {
        const msg = $ws;
        if (msg) {
            jobs = jobs.map(j =>
                j.id === msg.job_id
                    ? {
                        ...j,
                        status: msg.status,
                        channel_name: msg.channel_name ?? j.channel_name,
                        title: msg.title ?? j.title,
                        error: msg.error,
                        progress: msg.progress ?? (msg.status !== 'downloading' ? null : j.progress),
                      }
                    : j
            );
        }
    });

    // Unsubscribe pattern for the store
    let unsubWs: (() => void) | undefined;
    onMount(() => { unsubWs = ws.subscribe(() => {}); });
    onDestroy(() => unsubWs?.());

    async function handleSubmit() {
        submitError = '';
        submitting = true;
        try {
            const job = await submitJob(url);
            jobs = [job, ...jobs];
            url = '';
        } catch (e: unknown) {
            submitError = e instanceof Error ? e.message : 'Submit failed';
        } finally {
            submitting = false;
        }
    }

    const statusColour: Record<Job['status'], string> = {
        queued: '#888',
        downloading: '#4af',
        copying: '#fa4',
        done: '#4c4',
        failed: '#f44',
    };
</script>

<main>
    <form onsubmit={(e) => { e.preventDefault(); handleSubmit(); }}>
        <input type="url" bind:value={url} placeholder="https://www.youtube.com/watch?v=..." disabled={submitting} />
        <button type="submit" disabled={submitting || !url}>Add</button>
    </form>
    {#if submitError}<p class="error">{submitError}</p>{/if}

    <table>
        <thead>
            <tr><th>Status</th><th>Channel</th><th>Title</th><th>URL</th><th>Added</th></tr>
        </thead>
        <tbody>
            {#each jobs as job (job.id)}
                <tr>
                    <td style="color:{statusColour[job.status]}">
                        {job.status}
                        {#if job.status === 'downloading' && job.progress != null}
                            <span class="progress">{job.progress.toFixed(0)}%</span>
                        {/if}
                    </td>
                    <td>{job.channel_name ?? '—'}</td>
                    <td>
                        {job.title ?? '—'}
                        {#if job.error}<span class="error" title={job.error}> ⚠</span>{/if}
                    </td>
                    <td><a href={job.url} target="_blank" rel="noreferrer">link</a></td>
                    <td>{new Date(job.created_at).toLocaleString()}</td>
                </tr>
            {/each}
        </tbody>
    </table>
</main>

<style>
    main { padding: 1rem; font-family: sans-serif; }
    form { display: flex; gap: 0.5rem; margin-bottom: 1rem; }
    input[type=url] { flex: 1; padding: 0.4rem; }
    table { width: 100%; border-collapse: collapse; }
    th, td { text-align: left; padding: 0.4rem 0.6rem; border-bottom: 1px solid #333; }
    .error { color: red; }
    .progress { font-size: 0.85em; opacity: 0.8; margin-left: 0.3em; }
</style>
