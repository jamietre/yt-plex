<script lang="ts">
    import { logout } from '$lib/api';
    import { page } from '$app/stores';
    import type { Snippet } from 'svelte';

    let { children }: { children: Snippet } = $props();
    let isAdmin = $state(false);

    $effect(() => {
        fetch('/api/auth/me').then(r => { isAdmin = r.ok; });
    });

    async function handleLogout() {
        await logout();
        isAdmin = false;
        window.location.href = '/login';
    }

    function isActive(prefix: string) {
        return $page.url.pathname.startsWith(prefix);
    }
</script>

{#if $page.url.pathname !== '/login'}
<nav>
    <a href="/browse" class:active={isActive('/browse')}>Browse</a>
    <a href="/queue" class:active={isActive('/queue')}>Queue</a>
    {#if isAdmin}
        <a href="/admin" class:active={isActive('/admin')}>Admin</a>
        <button class="logout" onclick={handleLogout}>Log out</button>
    {/if}
</nav>
{/if}

{@render children()}

<style>
    nav {
        display: flex;
        gap: 0;
        background: #111;
        border-bottom: 1px solid #333;
        padding: 0 1rem;
        align-items: center;
    }
    nav a {
        color: #888;
        text-decoration: none;
        padding: 0.65rem 1rem;
        font-size: 0.9rem;
        border-bottom: 2px solid transparent;
        margin-bottom: -1px;
    }
    nav a.active { color: #4af; border-bottom-color: #4af; }
    nav a:hover:not(.active) { color: #ccc; }
    .logout {
        margin-left: auto;
        cursor: pointer;
        background: none;
        border: 1px solid #555;
        color: #888;
        padding: 0.25rem 0.75rem;
        border-radius: 4px;
        font-size: 0.85rem;
    }
</style>
