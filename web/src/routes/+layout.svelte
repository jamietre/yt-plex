<script lang="ts">
    import { goto } from '$app/navigation';
    import { page } from '$app/stores';
    import { logout, getProfileSession, clearProfileSession, type Profile } from '$lib/api';
    import { onMount } from 'svelte';
    import type { Snippet } from 'svelte';

    let { children }: { children: Snippet } = $props();
    let isAdmin = $state(false);
    let profile = $state<Profile | null>(null);
    let profileLoaded = $state(false);

    // Routes that don't need a profile
    const PUBLIC_ROUTES = ['/login', '/select-profile'];

    onMount(async () => {
        const [adminResp, profileData] = await Promise.allSettled([
            fetch('/api/auth/me'),
            getProfileSession(),
        ]);

        if (adminResp.status === 'fulfilled') {
            isAdmin = adminResp.value.ok;
        }
        if (profileData.status === 'fulfilled') {
            profile = profileData.value;
        }
        profileLoaded = true;

        // Guard: redirect to profile picker if no profile selected
        const path = $page.url.pathname;
        if (!profile && !PUBLIC_ROUTES.some(r => path.startsWith(r))) {
            goto('/select-profile');
        }
    });

    async function handleLogout() {
        await logout();
        isAdmin = false;
        window.location.href = '/login';
    }

    async function handleSwitchProfile() {
        await clearProfileSession();
        profile = null;
        goto('/select-profile');
    }

    function isActive(prefix: string) {
        return $page.url.pathname.startsWith(prefix);
    }
</script>

{#if $page.url.pathname !== '/login' && $page.url.pathname !== '/select-profile'}
<nav>
    <a href="/browse" class:active={isActive('/browse')}>Browse</a>
    <a href="/queue" class:active={isActive('/queue')}>Queue</a>
    {#if isAdmin}
        <a href="/admin" class:active={isActive('/admin')}>Admin</a>
    {/if}
    <div class="spacer"></div>
    {#if profile}
        <span class="profile-name">{profile.name}</span>
        <button class="switch-btn" onclick={handleSwitchProfile}>Switch</button>
    {/if}
    {#if isAdmin}
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
    .spacer { flex: 1; }
    .profile-name {
        font-size: 0.85rem;
        color: #888;
        padding: 0 0.5rem;
    }
    .switch-btn {
        cursor: pointer;
        background: none;
        border: 1px solid #444;
        color: #666;
        padding: 0.2rem 0.6rem;
        border-radius: 4px;
        font-size: 0.8rem;
        margin-right: 0.5rem;
    }
    .switch-btn:hover { color: #4af; border-color: #4af; }
    .logout {
        cursor: pointer;
        background: none;
        border: 1px solid #555;
        color: #888;
        padding: 0.25rem 0.75rem;
        border-radius: 4px;
        font-size: 0.85rem;
    }
</style>
