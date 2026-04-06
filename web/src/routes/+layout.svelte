<script lang="ts">
    import '$lib/styles/theme.css';
    import { goto } from '$app/navigation';
    import { page } from '$app/stores';
    import { logout, getProfileSession, clearProfileSession, type Profile } from '$lib/api';
    import { onMount } from 'svelte';
    import type { Snippet } from 'svelte';

    let { children }: { children: Snippet } = $props();
    let isAdmin = $state(false);
    let profile = $state<Profile | null>(null);
    let profileLoaded = $state(false);

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
    <a href="/browse" class="logo">yt-plex</a>
    <div class="nav-links">
        <a href="/browse" class:active={isActive('/browse')}>Browse</a>
        <a href="/queue"  class:active={isActive('/queue')}>Queue</a>
        {#if isAdmin}
            <a href="/admin" class:active={isActive('/admin')}>Admin</a>
        {/if}
    </div>
    <div class="nav-right">
        {#if profile}
            <span class="profile-name">{profile.name}</span>
            <button class="btn-switch" onclick={handleSwitchProfile}>Switch</button>
        {/if}
        {#if isAdmin}
            <button class="btn-logout" onclick={handleLogout}>Log out</button>
        {/if}
    </div>
</nav>
{/if}

{@render children()}

<style>
    nav {
        display: flex;
        align-items: center;
        gap: 0;
        background: var(--surface);
        border-bottom: 1px solid var(--border);
        padding: 0 20px;
        height: 48px;
        position: sticky;
        top: 0;
        z-index: 100;
    }

    .logo {
        font-family: var(--font-display);
        font-size: 18px;
        font-weight: 700;
        color: var(--amber);
        text-decoration: none;
        letter-spacing: 0.5px;
        margin-right: 16px;
        flex-shrink: 0;
    }
    .logo:hover { color: var(--amber-glow); }

    .nav-links {
        display: flex;
        align-items: center;
        flex: 1;
    }

    .nav-links a {
        color: var(--text-2);
        text-decoration: none;
        font-size: 13px;
        font-weight: 500;
        padding: 6px 12px;
        border-radius: var(--radius);
        transition: color 0.15s, background 0.15s;
    }
    .nav-links a:hover:not(.active) { color: var(--text); }
    .nav-links a.active {
        color: var(--amber);
        background: rgba(232, 160, 32, 0.08);
    }

    .nav-right {
        display: flex;
        align-items: center;
        gap: 8px;
    }

    .profile-name {
        font-size: 12px;
        color: var(--text-2);
    }

    .btn-switch {
        background: transparent;
        border: 1px solid var(--border-2);
        color: var(--text-2);
        padding: 3px 9px;
        border-radius: var(--radius);
        font-size: 11px;
        font-weight: 500;
        cursor: pointer;
        transition: border-color 0.15s, color 0.15s;
        font-family: var(--font-ui);
    }
    .btn-switch:hover { border-color: var(--amber); color: var(--amber); }

    .btn-logout {
        background: transparent;
        border: 1px solid var(--border-2);
        color: var(--text-2);
        padding: 3px 9px;
        border-radius: var(--radius);
        font-size: 11px;
        font-weight: 500;
        cursor: pointer;
        transition: border-color 0.15s, color 0.15s;
        font-family: var(--font-ui);
    }
    .btn-logout:hover { border-color: var(--red); color: var(--red); }
</style>
