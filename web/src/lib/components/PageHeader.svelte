<script lang="ts">
    import type { Snippet } from 'svelte';

    let {
        title,
        subtitle,
        actions,
    }: {
        title: string;
        subtitle?: string;
        actions?: Snippet;
    } = $props();
</script>

<div class="page-header">
    <div class="left">
        <h1 class="title">{title}</h1>
        {#if subtitle}
            <span class="subtitle">{subtitle}</span>
        {/if}
    </div>
    {#if actions}
        <div class="actions">
            {@render actions()}
        </div>
    {/if}
</div>

<style>
    .page-header {
        display: flex;
        align-items: baseline;
        gap: 12px;
        margin-bottom: 20px;
    }
    .left { display: flex; align-items: baseline; gap: 10px; flex: 1; min-width: 0; }
    .title {
        font-family: var(--font-display);
        font-size: 22px;
        font-weight: 700;
        color: var(--text);
        line-height: 1.2;
        margin: 0;
    }
    .subtitle {
        font-size: 12px;
        color: var(--text-3);
        flex-shrink: 0;
    }
    .actions { display: flex; align-items: center; gap: 8px; flex-shrink: 0; }
</style>
