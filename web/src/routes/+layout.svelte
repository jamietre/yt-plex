<script lang="ts">
    import '$lib/styles/theme.css';
    import { goto, afterNavigate } from '$app/navigation';
    import { page } from '$app/stores';
    import { logout, getProfileSession, clearProfileSession, type Profile } from '$lib/api';
    import { navSearch } from '$lib/navSearch';
    import { wsConnect, wsDisconnect } from '$lib/ws';
    import { loadPrefs, applyPrefs } from '$lib/prefs';
    import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
    import Toaster from '$lib/components/Toaster.svelte';
    import { onMount, onDestroy } from 'svelte';
    import type { Snippet } from 'svelte';

    let { children }: { children: Snippet } = $props();
    let isAdmin = $state(false);
    let profile = $state<Profile | null>(null);
    let profileLoaded = $state(false);
    let menuOpen = $state(false);

    function toggleMenu() { menuOpen = !menuOpen; }
    function closeMenu() { menuOpen = false; }

    $effect(() => {
        if (!menuOpen) return;
        function onClickOutside(e: MouseEvent) {
            if (!(e.target as Element).closest('.avatar-wrap')) closeMenu();
        }
        window.addEventListener('click', onClickOutside, true);
        return () => window.removeEventListener('click', onClickOutside, true);
    });

    const PUBLIC_ROUTES = ['/login', '/select-profile'];

    onMount(() => wsConnect());
    onDestroy(() => wsDisconnect());

    onMount(async () => {
        applyPrefs(loadPrefs());

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

    afterNavigate(async ({ from }) => {
        if (from?.url?.pathname === '/select-profile') {
            const profileData = await getProfileSession();
            if (profileData) profile = profileData;
        }
    });
</script>

{#if $page.url.pathname !== '/login' && $page.url.pathname !== '/select-profile'}
<nav>
    <a href="/browse" class="logo">yt-plex</a>
    <div class="nav-center">
        <div class="nav-links">
            <a href="/browse" class:active={isActive('/browse')}>Browse</a>
            <a href="/queue"  class:active={isActive('/queue')}>Queue</a>
            {#if isAdmin}
                <a href="/admin" class:active={isActive('/admin')}>Admin</a>
            {/if}
        </div>
        {#if $navSearch}
            <input
                class="nav-search"
                type="search"
                placeholder={$navSearch.placeholder}
                value={$navSearch.value}
                oninput={(e) => $navSearch?.onInput((e.target as HTMLInputElement).value)}
            />
        {/if}
    </div>
    <div class="nav-right">
        {#if profile || isAdmin}
            <div class="avatar-wrap">
                <button class="avatar-trigger" onclick={toggleMenu} aria-label="Profile menu">
                    {#if profile}
                        <span class="profile-label">{profile.name}</span>
                    {/if}
                    <div class="avatar">
                        {#if profile}
                            {profile.name[0].toUpperCase()}
                        {:else}
                            ⚙
                        {/if}
                    </div>
                </button>
                {#if menuOpen}
                    <div class="dropdown">
                        {#if profile}
                            <span class="dropdown-profile">{profile.name}</span>
                        {/if}
                        <button class="dropdown-item" onclick={() => { closeMenu(); handleSwitchProfile(); }}>
                            Switch profile
                        </button>
                        <a class="dropdown-item" href="/preferences" onclick={closeMenu}>
                            Settings
                        </a>
                        {#if isAdmin}
                            <button class="dropdown-item danger" onclick={() => { closeMenu(); handleLogout(); }}>
                                Log out
                            </button>
                        {/if}
                    </div>
                {/if}
            </div>
        {/if}
    </div>
</nav>
{/if}

{@render children()}

<ConfirmDialog />
<Toaster />

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

    .nav-center {
        display: flex;
        align-items: center;
        gap: 8px;
        flex: 1;
    }

    .nav-links {
        display: flex;
        align-items: center;
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

    .nav-search {
        margin: 0 12px 0 4px;
        padding: 4px 12px;
        background: var(--surface-2);
        border: 1px solid var(--border);
        color: var(--text);
        border-radius: 20px;
        font-size: 12px;
        font-family: var(--font-ui);
        width: 200px;
        outline: none;
        transition: border-color 0.15s;
    }
    .nav-search:focus { border-color: var(--amber); }
    .nav-search::placeholder { color: var(--text-3); }

    .nav-right {
        display: flex;
        align-items: center;
    }

    .avatar-wrap {
        position: relative;
    }

    .avatar-trigger {
        display: flex;
        align-items: center;
        gap: 8px;
        background: transparent;
        border: none;
        cursor: pointer;
        padding: 0;
    }
    .avatar-trigger:hover .avatar { background: var(--amber-glow); }
    .avatar-trigger:hover .profile-label { color: var(--text); }

    .profile-label {
        font-size: 12px;
        color: var(--text-2);
        font-family: var(--font-ui);
        transition: color 0.15s;
    }

    .avatar {
        width: 30px;
        height: 30px;
        border-radius: 50%;
        background: var(--amber);
        color: #000;
        font-size: 13px;
        font-weight: 700;
        font-family: var(--font-display);
        display: flex;
        align-items: center;
        justify-content: center;
        transition: background 0.15s;
        flex-shrink: 0;
    }

    .dropdown {
        position: absolute;
        top: calc(100% + 4px);
        right: 0;
        background: var(--surface-2);
        border: 1px solid var(--border-2);
        border-radius: var(--radius);
        min-width: 140px;
        padding: 4px 0;
        z-index: 200;
        box-shadow: 0 4px 16px rgba(0,0,0,0.5);
    }

    .dropdown-profile {
        display: block;
        font-size: 11px;
        color: var(--text-3);
        padding: 6px 12px 4px;
        border-bottom: 1px solid var(--border);
        margin-bottom: 4px;
    }

    .dropdown-item {
        display: block;
        width: 100%;
        text-align: left;
        background: transparent;
        border: none;
        color: var(--text-2);
        font-size: 13px;
        font-family: var(--font-ui);
        padding: 7px 12px;
        cursor: pointer;
        transition: background 0.1s, color 0.1s;
        text-decoration: none;
    }
    .dropdown-item:hover { background: var(--surface-3); color: var(--text); }
    .dropdown-item.danger:hover { color: var(--red); }
</style>
