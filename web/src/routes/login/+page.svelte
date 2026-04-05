<script lang="ts">
    import { startDeviceLogin, pollDeviceAuth, type DeviceLoginResponse } from '$lib/api';

    type Phase = 'idle' | 'waiting' | 'done' | 'error';

    let phase = $state<Phase>('idle');
    let deviceInfo = $state<DeviceLoginResponse | null>(null);
    let errorMsg = $state('');
    let pollTimer: ReturnType<typeof setTimeout> | null = null;
    let networkErrors = 0;
    const MAX_NETWORK_ERRORS = 5;

    function cancelPoll() {
        if (pollTimer !== null) {
            clearTimeout(pollTimer);
            pollTimer = null;
        }
    }

    async function startLogin() {
        cancelPoll();
        phase = 'idle';
        errorMsg = '';
        deviceInfo = null;
        networkErrors = 0;
        try {
            deviceInfo = await startDeviceLogin();
            phase = 'waiting';
            schedulePoll(deviceInfo.interval);
        } catch {
            phase = 'error';
            errorMsg = 'Failed to start sign-in. Please try again.';
        }
    }

    function schedulePoll(intervalSecs: number) {
        pollTimer = setTimeout(doPoll, intervalSecs * 1000);
    }

    async function doPoll() {
        if (!deviceInfo) return;
        try {
            const result = await pollDeviceAuth(deviceInfo.poll_token);
            networkErrors = 0;
            if (result.status === 'pending') {
                schedulePoll(result.interval ?? deviceInfo.interval);
            } else if (result.status === 'done') {
                phase = 'done';
                window.location.href = '/';
            } else if (result.status === 'expired') {
                phase = 'error';
                errorMsg = 'Sign-in timed out. Please try again.';
            } else {
                phase = 'error';
                errorMsg = result.message ?? 'Sign-in failed. Please try again.';
            }
        } catch {
            networkErrors++;
            if (networkErrors >= MAX_NETWORK_ERRORS) {
                phase = 'error';
                errorMsg = 'Lost connection to server. Please try again.';
            } else {
                schedulePoll(deviceInfo.interval);
            }
        }
    }
</script>

<main>
    <h2>Sign in</h2>

    {#if phase === 'idle' || phase === 'error'}
        {#if errorMsg}<p class="error">{errorMsg}</p>{/if}
        <button onclick={startLogin}>Sign in with Google</button>

    {:else if phase === 'waiting' && deviceInfo}
        <p>Click the link below to sign in:</p>
        <p><a href="{deviceInfo.verification_url}?user_code={deviceInfo.user_code}" target="_blank" rel="noreferrer">Sign in with Google ↗</a></p>
        <p class="code">{deviceInfo.user_code}</p>
        <p class="hint">Waiting for authorisation…</p>

    {:else if phase === 'done'}
        <p>Signed in! Redirecting…</p>
    {/if}
</main>

<style>
    main {
        max-width: 400px;
        margin: 6rem auto;
        padding: 2rem;
        font-family: sans-serif;
        text-align: center;
        border: 1px solid #333;
        border-radius: 8px;
        background: #1a1a1a;
        color: #fff;
    }
    h2 { margin-bottom: 1.5rem; }
    button {
        padding: 0.6rem 1.4rem;
        background: #fff;
        color: #333;
        border: none;
        border-radius: 4px;
        cursor: pointer;
        font-weight: 500;
        font-size: 1rem;
    }
    button:hover { background: #e8e8e8; }
    .code {
        font-size: 2rem;
        font-weight: bold;
        letter-spacing: 0.2em;
        margin: 1rem 0;
        font-family: monospace;
    }
    .hint { color: #888; font-size: 0.9rem; }
    .error { color: #f66; }
    a { color: #4af; }
</style>
