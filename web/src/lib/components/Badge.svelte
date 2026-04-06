<script lang="ts">
    import type { VideoStatus } from '$lib/api';

    let { status }: { status: VideoStatus } = $props();

    const label: Record<VideoStatus, string> = {
        new:         'New',
        in_progress: '↓',
        downloaded:  '✓ Plex',
        ignored:     'Ignored',
    };
</script>

<span class="badge badge-{status}">{label[status]}</span>

<style>
    .badge {
        display: inline-block;
        font-family: var(--font-ui);
        font-size: 9px;
        font-weight: 700;
        padding: 2px 7px;
        border-radius: 3px;
        letter-spacing: 0.5px;
        text-transform: uppercase;
    }
    .badge-new        { background: rgba(232,160,32,0.15); color: var(--amber);  border: 1px solid rgba(232,160,32,0.3); }
    .badge-in_progress{ background: rgba(232,144,58,0.15); color: var(--orange); border: 1px solid rgba(232,144,58,0.3); }
    .badge-downloaded { background: rgba(76,175,118,0.15); color: var(--green);  border: 1px solid rgba(76,175,118,0.3); }
    .badge-ignored    { background: rgba(85,80,80,0.3);    color: var(--text-3); border: 1px solid rgba(85,80,80,0.4);  }
</style>
