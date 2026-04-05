<script lang="ts">
    import { logout } from '$lib/api';
    import { page } from '$app/stores';
    import type { Snippet } from 'svelte';

    let { children }: { children: Snippet } = $props();

    async function handleLogout() {
        await logout();
        window.location.href = '/login';
    }
</script>

{#if $page.url.pathname !== '/login'}
<nav>
    <a href="/">Jobs</a>
    <a href="/settings">Settings</a>
    <button onclick={handleLogout}>Log out</button>
</nav>
{/if}

{@render children()}

<style>
    nav { display: flex; gap: 1rem; padding: 0.75rem 1rem; background: #1a1a1a; color: white; align-items: center; }
    nav a { color: white; text-decoration: none; }
    nav button { margin-left: auto; cursor: pointer; background: none; border: 1px solid #888; color: white; padding: 0.25rem 0.75rem; border-radius: 4px; }
</style>
