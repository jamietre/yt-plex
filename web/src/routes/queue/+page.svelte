<script lang="ts">
    import { onMount } from 'svelte';
    import { listJobs, submitJob, type Job } from '$lib/api';
    import { wsMessages } from '$lib/ws';
    import Badge from '$lib/components/Badge.svelte';
    import Button from '$lib/components/Button.svelte';
    import PageHeader from '$lib/components/PageHeader.svelte';
    import EmptyState from '$lib/components/EmptyState.svelte';

    let jobs = $state<Job[]>([]);
    let url = $state('');
    let submitError = $state('');
    let submitting = $state(false);

    onMount(async () => {
        try { jobs = await listJobs(); } catch { /* ignore */ }
    });

    $effect(() => {
        const msg = $wsMessages;
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

    // Map Job status → VideoStatus for Badge (only new/in_progress/downloaded map cleanly;
    // queued/copying map to in_progress; failed gets a custom inline display)
    function jobStatusBadge(status: Job['status']): 'new' | 'in_progress' | 'downloaded' | 'ignored' {
        if (status === 'done')       return 'downloaded';
        if (status === 'failed')     return 'ignored';
        return 'in_progress';
    }

    const statusLabel: Record<Job['status'], string> = {
        queued:      'Queued',
        downloading: 'Downloading',
        copying:     'Copying',
        done:        'Done',
        failed:      'Failed',
    };
</script>

<div class="page">
    <PageHeader title="Download Queue" />

    <form onsubmit={(e) => { e.preventDefault(); handleSubmit(); }} class="submit-form">
        <input
            type="url"
            bind:value={url}
            placeholder="https://www.youtube.com/watch?v=…"
            disabled={submitting}
            class="url-input"
        />
        <Button type="submit" variant="primary" disabled={submitting || !url}>
            {submitting ? 'Queuing…' : 'Add URL'}
        </Button>
    </form>
    {#if submitError}<p class="msg-error">{submitError}</p>{/if}

    {#if jobs.length === 0}
        <EmptyState message="No downloads yet." />
    {:else}
        <table class="data-table">
            <thead>
                <tr>
                    <th>Status</th>
                    <th>Channel</th>
                    <th>Title</th>
                    <th>Added</th>
                </tr>
            </thead>
            <tbody>
                {#each jobs as job (job.id)}
                    <tr>
                        <td class="td-status">
                            <Badge status={jobStatusBadge(job.status)} />
                            {#if job.status === 'downloading' && job.progress != null}
                                <span class="progress">{job.progress.toFixed(0)}%</span>
                            {/if}
                            {#if job.status !== 'done' && job.status !== 'failed'}
                                <span class="status-label">{statusLabel[job.status]}</span>
                            {/if}
                            {#if job.error}
                                <span class="error-indicator" title={job.error}>⚠</span>
                            {/if}
                        </td>
                        <td>{job.channel_name ?? '—'}</td>
                        <td class="td-title">{job.title ?? '—'}</td>
                        <td class="td-date">{new Date(job.created_at).toLocaleString()}</td>
                    </tr>
                {/each}
            </tbody>
        </table>
    {/if}
</div>

<style>
    .page { padding: 28px 24px; max-width: 900px; }

    .submit-form {
        display: flex;
        gap: 8px;
        margin-bottom: 8px;
        align-items: center;
    }
    .url-input {
        flex: 1;
        font-family: var(--font-ui);
        font-size: 13px;
        background: var(--surface-2);
        border: 1px solid var(--border);
        color: var(--text);
        border-radius: var(--radius);
        padding: 7px 11px;
        outline: none;
        transition: border-color 0.15s;
    }
    .url-input:focus { border-color: var(--amber); }
    .url-input::placeholder { color: var(--text-3); }
    .url-input:disabled { opacity: 0.5; }

    .msg-error { color: var(--red); font-size: 12px; margin: 0 0 16px; }

    .data-table {
        width: 100%;
        border-collapse: collapse;
        font-size: 12px;
        margin-top: 20px;
    }
    .data-table th {
        text-align: left;
        padding: 6px 10px;
        color: var(--text-3);
        border-bottom: 1px solid var(--border);
        font-size: 10px;
        font-weight: 700;
        text-transform: uppercase;
        letter-spacing: 0.5px;
    }
    .data-table td {
        padding: 9px 10px;
        border-bottom: 1px solid var(--border);
        color: var(--text-2);
        vertical-align: middle;
    }

    .td-status { display: flex; align-items: center; gap: 6px; white-space: nowrap; }
    .status-label { font-size: 11px; color: var(--text-3); }
    .progress { font-size: 11px; color: var(--orange); font-weight: 600; }
    .error-indicator { color: var(--red); font-size: 13px; cursor: help; }

    .td-title { color: var(--text); max-width: 340px; }
    .td-date  { white-space: nowrap; font-size: 11px; color: var(--text-3); }
</style>
