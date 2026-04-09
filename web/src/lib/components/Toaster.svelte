<!-- web/src/lib/components/Toaster.svelte -->
<script lang="ts">
    import { toasts } from '$lib/toast';
</script>

{#if $toasts.length > 0}
    <div class="toaster">
        {#each $toasts as t (t.id)}
            <div class="toast toast-{t.variant}">
                <span class="icon">
                    {#if t.variant === 'error'}✕{:else}✓{/if}
                </span>
                <span class="message">{t.message}</span>
            </div>
        {/each}
    </div>
{/if}

<style>
    .toaster {
        position: fixed;
        bottom: 20px;
        right: 20px;
        display: flex;
        flex-direction: column;
        gap: 8px;
        z-index: 1100;
        pointer-events: none;
    }

    .toast {
        display: flex;
        align-items: center;
        gap: 10px;
        padding: 11px 16px;
        border-radius: var(--radius-lg);
        font-family: var(--font-ui);
        font-size: 13px;
        box-shadow: 0 4px 20px rgba(0, 0, 0, 0.5);
        animation: slide-in 0.2s ease;
    }

    .toast-success {
        background: #1e1608;
        border: 1px solid #3d2e0a;
        color: var(--amber-glow);
    }

    .toast-error {
        background: #1e0808;
        border: 1px solid #3d0a0a;
        color: var(--red);
    }

    .toast-info {
        background: var(--surface-2);
        border: 1px solid var(--border-2);
        color: var(--text-2);
    }

    .icon {
        font-size: 14px;
        flex-shrink: 0;
    }

    .toast-success .icon { color: var(--amber); }
    .toast-error .icon   { color: var(--red); }
    .toast-info .icon    { color: var(--text-3); }

    @keyframes slide-in {
        from { opacity: 0; transform: translateX(16px); }
        to   { opacity: 1; transform: translateX(0); }
    }
</style>
