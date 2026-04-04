<script lang="ts">
    import { login } from '$lib/api';

    let password = $state('');
    let error = $state('');
    let loading = $state(false);

    async function handleSubmit() {
        error = '';
        loading = true;
        try {
            await login(password);
            window.location.href = '/';
        } catch (e: unknown) {
            error = e instanceof Error ? e.message : 'Login failed';
        } finally {
            loading = false;
        }
    }
</script>

<main>
    <h1>yt-plex</h1>
    <form onsubmit={(e) => { e.preventDefault(); handleSubmit(); }}>
        <label>
            Admin password
            <input type="password" bind:value={password} disabled={loading} />
        </label>
        {#if error}<p class="error">{error}</p>{/if}
        <button type="submit" disabled={loading || !password}>
            {loading ? 'Logging in…' : 'Log in'}
        </button>
    </form>
</main>

<style>
    main { max-width: 360px; margin: 10vh auto; font-family: sans-serif; }
    form { display: flex; flex-direction: column; gap: 1rem; }
    .error { color: red; }
</style>
