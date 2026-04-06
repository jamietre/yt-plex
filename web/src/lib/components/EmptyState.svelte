<script lang="ts">
    let { message }: { message: string } = $props();
</script>

<p class="empty">{message}</p>

<style>
    .empty {
        color: var(--text-3);
        font-style: italic;
        font-size: 13px;
        text-align: center;
        padding: 2rem;
        grid-column: 1 / -1;
    }
</style>
