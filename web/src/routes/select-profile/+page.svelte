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

<div class="page">
    <div class="card">
        <h1 class="heading">Who's watching?</h1>

        {#if loading}
            <p class="hint">Loading profiles…</p>
        {:else if error}
            <p class="msg-error">{error}</p>
        {:else if profiles.length === 0 && !adminProfile}
            <p class="hint">No profiles yet. Ask an admin to create one, or log in as admin below.</p>
        {:else}
            <div class="grid">
                {#each profiles as profile}
                    <button
                        class="profile-btn"
                        disabled={selecting !== null}
                        onclick={() => selectProfile(profile)}
                    >
                        <div class="avatar">{profile.name[0].toUpperCase()}</div>
                        <span class="profile-name">{profile.name}</span>
                        {#if selecting === profile.id}
                            <span class="selecting">…</span>
                        {/if}
                    </button>
                {/each}
                {#if adminProfile}
                    <button
                        class="profile-btn admin-btn"
                        disabled={selecting !== null}
                        onclick={() => selectProfile(adminProfile!)}
                    >
                        <div class="avatar admin-avatar">⚙</div>
                        <span class="profile-name">Admin</span>
                        {#if selecting === adminProfile.id}
                            <span class="selecting">…</span>
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
    </div>
</div>

<style>
    .page {
        min-height: 100vh;
        display: flex;
        align-items: center;
        justify-content: center;
        padding: 2rem;
    }
    .card {
        width: 100%;
        max-width: 560px;
        text-align: center;
    }
    .heading {
        font-family: var(--font-display);
        font-size: 28px;
        font-weight: 700;
        color: var(--text);
        margin: 0 0 2rem;
    }
    .hint     { color: var(--text-2); font-size: 13px; font-style: italic; }
    .msg-error{ color: var(--red);    font-size: 13px; }

    .grid {
        display: flex;
        flex-wrap: wrap;
        gap: 12px;
        justify-content: center;
        margin-bottom: 2rem;
    }

    .profile-btn {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 8px;
        background: var(--surface-2);
        border: 1px solid var(--border);
        border-radius: var(--radius-lg);
        padding: 20px 24px;
        cursor: pointer;
        min-width: 120px;
        transition: border-color 0.15s, background 0.15s;
        font-family: var(--font-ui);
    }
    .profile-btn:hover:not(:disabled) { background: var(--surface-3); border-color: var(--amber); }
    .profile-btn:disabled { opacity: 0.6; cursor: default; }

    .avatar {
        width: 52px;
        height: 52px;
        border-radius: 50%;
        background: var(--amber);
        color: #000;
        font-size: 1.3rem;
        font-weight: 700;
        display: flex;
        align-items: center;
        justify-content: center;
        font-family: var(--font-display);
    }
    .admin-btn { border-color: var(--border-2); }
    .admin-btn:hover:not(:disabled) { border-color: var(--orange); }
    .admin-avatar { background: var(--surface-3); color: var(--text-2); font-size: 1.1rem; border: 1px solid var(--border-2); }

    .profile-name { color: var(--text); font-size: 13px; font-weight: 500; }
    .selecting    { color: var(--text-3); font-size: 12px; }

    .login-row { margin-top: 1.5rem; }
    .login-link { color: var(--text-3); font-size: 12px; text-decoration: none; transition: color 0.15s; }
    .login-link:hover { color: var(--amber); }
</style>
