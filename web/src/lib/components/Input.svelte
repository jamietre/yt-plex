<script lang="ts">
    let {
        type = 'text',
        placeholder = '',
        value = $bindable(''),
        disabled = false,
        class: extraClass = '',
        oninput,
        onkeydown,
    }: {
        type?: 'text' | 'url' | 'search';
        placeholder?: string;
        value?: string;
        disabled?: boolean;
        class?: string;
        oninput?: (e: Event) => void;
        onkeydown?: (e: KeyboardEvent) => void;
    } = $props();
</script>

<input
    {type}
    {placeholder}
    bind:value
    {disabled}
    class="input {extraClass}"
    {oninput}
    {onkeydown}
/>

<style>
    .input {
        font-family: var(--font-ui);
        font-size: 13px;
        background: var(--surface-2);
        border: 1px solid var(--border);
        color: var(--text);
        border-radius: var(--radius);
        padding: 7px 11px;
        outline: none;
        transition: border-color 0.15s;
        width: 100%;
    }
    .input:focus  { border-color: var(--amber); }
    .input::placeholder { color: var(--text-3); }
    .input:disabled { opacity: 0.5; cursor: default; }
</style>
