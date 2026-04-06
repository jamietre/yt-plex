<script lang="ts">
    import { goto } from '$app/navigation';
    import { listProfiles, setProfileSession, getAdminProfile, type Profile } from '$lib/api';
    import { onMount } from 'svelte';

    let profiles = $state<Profile[]>([]);
    let adminProfile = $state<Profile | null>(null);
    let loading = $state(true);
    let error = $state('');
    let selecting = $state<number | null>(null);

    onMount(async () => {
        try {
            [profiles, adminProfile] = await Promise.all([listProfiles(), getAdminProfile()]);
        } catch (e) {
            error = e instanceof Error ? e.message : 'Failed to load profiles';
        } finally {
            loading = false;
        }
    });

    async function selectProfile(profile: Profile) {
        selecting = profile.id;
        try {
            await setProfileSession(profile.id);
            goto('/browse');
        } catch (e) {
            error = e instanceof Error ? e.message : 'Failed to select profile';
            selecting = null;
        }
    }
</script>

<main>
    <h1>Who's watching?</h1>

    {#if loading}
        <p class="hint">Loading profiles…</p>
    {:else if error}
        <p class="error">{error}</p>
    {:else if profiles.length === 0 && !adminProfile}
        <p class="hint">No profiles yet. Ask an admin to create one, or log in as admin below.</p>
    {:else}
        <div class="grid">
            {#each profiles as profile}
                <button
                    class="card"
                    disabled={selecting !== null}
                    onclick={() => selectProfile(profile)}
                >
                    <div class="avatar">{profile.name[0].toUpperCase()}</div>
                    <span class="name">{profile.name}</span>
                    {#if selecting === profile.id}
                        <span class="loading-dot">…</span>
                    {/if}
                </button>
            {/each}
            {#if adminProfile}
                <button
                    class="card admin-card"
                    disabled={selecting !== null}
                    onclick={() => selectProfile(adminProfile!)}
                >
                    <div class="avatar admin-avatar">⚙</div>
                    <span class="name">Admin</span>
                    {#if selecting === adminProfile.id}
                        <span class="loading-dot">…</span>
                    {/if}
                </button>
            {/if}
        </div>
    {/if}

    {#if !adminProfile}
        <div class="login-row">
            <a href="/login" class="login-link">Admin login ↗</a>
        </div>
    {/if}
</main>

<style>
    main {
        max-width: 600px;
        margin: 4rem auto;
        padding: 1rem;
        font-family: sans-serif;
        text-align: center;
    }
    h1 {
        color: #ddd;
        font-size: 1.6rem;
        margin-bottom: 2rem;
    }
    .grid {
        display: flex;
        flex-wrap: wrap;
        gap: 1rem;
        justify-content: center;
        margin-bottom: 2rem;
    }
    .card {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 0.5rem;
        background: #1a1a2e;
        border: 1px solid #333;
        border-radius: 8px;
        padding: 1.5rem 2rem;
        cursor: pointer;
        min-width: 120px;
        transition: border-color 0.15s, background 0.15s;
    }
    .card:hover:not(:disabled) {
        background: #22223a;
        border-color: #4af;
    }
    .card:disabled { opacity: 0.6; cursor: default; }
    .avatar {
        width: 52px;
        height: 52px;
        border-radius: 50%;
        background: #4af;
        color: #000;
        font-size: 1.4rem;
        font-weight: 700;
        display: flex;
        align-items: center;
        justify-content: center;
    }
    .name { color: #ccc; font-size: 0.95rem; }
    .loading-dot { color: #888; font-size: 0.8rem; }
    .admin-card { border-color: #555; }
    .admin-card:hover:not(:disabled) { border-color: #fa4; }
    .admin-avatar { background: #555; color: #ddd; font-size: 1.2rem; }
    .login-row { margin-top: 1.5rem; }
    .login-link { color: #666; font-size: 0.85rem; text-decoration: none; }
    .login-link:hover { color: #4af; text-decoration: underline; }
    .hint { color: #888; font-style: italic; }
    .error { color: #f44; }
</style>
