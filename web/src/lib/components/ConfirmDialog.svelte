<!-- web/src/lib/components/ConfirmDialog.svelte -->
<script lang="ts">
    import { confirmState } from '$lib/confirm';

    function respond(value: boolean) {
        confirmState.update(s => {
            s?.resolve(value);
            return null;
        });
    }
</script>

{#if $confirmState}
    <svelte:window onkeydown={(e) => { if (e.key === 'Escape') respond(false); }} />
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="backdrop" onclick={() => respond(false)}>
        <div class="dialog" role="dialog" aria-modal="true"
             aria-labelledby="confirm-title" aria-describedby="confirm-msg"
             onclick={(e) => e.stopPropagation()}>
            <p class="title" id="confirm-title">{$confirmState.title}</p>
            <p class="message" id="confirm-msg">{$confirmState.message}</p>
            <div class="actions">
                <button class="btn-cancel" onclick={() => respond(false)} autofocus>
                    {$confirmState.cancelLabel ?? 'Cancel'}
                </button>
                <button class="btn-confirm" onclick={() => respond(true)}>
                    {$confirmState.confirmLabel ?? 'Confirm'}
                </button>
            </div>
        </div>
    </div>
{/if}

<style>
    .backdrop {
        position: fixed;
        inset: 0;
        background: rgba(0, 0, 0, 0.65);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 1000;
    }

    .dialog {
        background: var(--surface-2);
        border: 1px solid var(--border-2);
        border-radius: var(--radius-lg);
        padding: 24px;
        width: 320px;
        box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
    }

    .title {
        font-size: 15px;
        font-weight: 600;
        color: var(--text);
        margin: 0 0 8px;
    }

    .message {
        font-size: 13px;
        color: var(--text-2);
        line-height: 1.5;
        margin: 0 0 20px;
    }

    .actions {
        display: flex;
        gap: 8px;
        justify-content: flex-end;
    }

    .btn-cancel, .btn-confirm {
        font-family: var(--font-ui);
        font-weight: 600;
        font-size: 12px;
        padding: 5px 14px;
        border-radius: var(--radius);
        cursor: pointer;
        border: 1px solid transparent;
        line-height: 1;
        transition: background 0.15s, border-color 0.15s, color 0.15s;
    }

    .btn-cancel {
        background: transparent;
        border-color: transparent;
        color: var(--text-2);
    }
    .btn-cancel:hover { color: var(--text); }

    .btn-confirm {
        background: var(--amber);
        border-color: var(--amber);
        color: #000;
    }
    .btn-confirm:hover { background: var(--amber-glow); border-color: var(--amber-glow); }
</style>
