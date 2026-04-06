<script lang="ts">
    import type { Snippet } from 'svelte';

    type Variant = 'primary' | 'secondary' | 'danger' | 'ghost';
    type Size = 'md' | 'sm';

    let {
        variant = 'secondary',
        size = 'md',
        disabled = false,
        type = 'button',
        onclick,
        children,
    }: {
        variant?: Variant;
        size?: Size;
        disabled?: boolean;
        type?: 'button' | 'submit' | 'reset';
        onclick?: (e: MouseEvent) => void;
        children: Snippet;
    } = $props();
</script>

<button
    {type}
    {disabled}
    class="btn btn-{variant} size-{size}"
    {onclick}
>
    {@render children()}
</button>

<style>
    .btn {
        font-family: var(--font-ui);
        font-weight: 600;
        border-radius: var(--radius);
        cursor: pointer;
        letter-spacing: 0.3px;
        transition: background 0.15s, border-color 0.15s, color 0.15s;
        border: 1px solid transparent;
        line-height: 1;
        white-space: nowrap;
    }
    .size-md { font-size: 13px; padding: 7px 15px; }
    .size-sm { font-size: 11px; padding: 4px 10px; }

    .btn-primary { background: var(--amber); color: #000; border-color: var(--amber); }
    .btn-primary:hover:not(:disabled) { background: var(--amber-glow); border-color: var(--amber-glow); }

    .btn-secondary { background: transparent; border-color: var(--border-2); color: var(--text-2); }
    .btn-secondary:hover:not(:disabled) { border-color: var(--amber); color: var(--amber); }

    .btn-danger { background: transparent; border-color: #5a2020; color: var(--red); }
    .btn-danger:hover:not(:disabled) { border-color: var(--red); }

    .btn-ghost { background: transparent; border-color: transparent; color: var(--text-2); }
    .btn-ghost:hover:not(:disabled) { color: var(--text); }

    .btn:disabled { opacity: 0.45; cursor: default; }
</style>
