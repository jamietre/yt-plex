<script lang="ts">
    import { startDeviceLogin, pollDeviceAuth, type DeviceLoginResponse } from '$lib/api';
    import Button from '$lib/components/Button.svelte';

    type Phase = 'idle' | 'waiting' | 'done' | 'error';

    let phase = $state<Phase>('idle');
    let deviceInfo = $state<DeviceLoginResponse | null>(null);
    let errorMsg = $state('');
    let pollTimer: ReturnType<typeof setTimeout> | null = null;
    let networkErrors = 0;
    const MAX_NETWORK_ERRORS = 5;

    function cancelPoll() {
        if (pollTimer !== null) { clearTimeout(pollTimer); pollTimer = null; }
    }

    async function startLogin() {
        cancelPoll();
        phase = 'idle'; errorMsg = ''; deviceInfo = null; networkErrors = 0;
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
                phase = 'error'; errorMsg = 'Sign-in timed out. Please try again.';
            } else {
                phase = 'error'; errorMsg = result.message ?? 'Sign-in failed. Please try again.';
            }
        } catch {
            networkErrors++;
            if (networkErrors >= MAX_NETWORK_ERRORS) {
                phase = 'error'; errorMsg = 'Lost connection to server. Please try again.';
            } else {
                schedulePoll(deviceInfo.interval);
            }
        }
    }
</script>

<div class="page">
    <div class="card">
        <h1 class="heading">yt-plex</h1>
        <p class="sub">Admin sign-in</p>

        {#if phase === 'idle' || phase === 'error'}
            {#if errorMsg}<p class="msg-error">{errorMsg}</p>{/if}
            <Button variant="primary" onclick={startLogin}>Sign in with Google</Button>

        {:else if phase === 'waiting' && deviceInfo}
            <p class="instruction">Click the link below to sign in:</p>
            <a
                class="auth-link"
                href="{deviceInfo.verification_url}?user_code={deviceInfo.user_code}"
                target="_blank"
                rel="noreferrer"
            >
                Sign in with Google ↗
            </a>
            <div class="user-code">{deviceInfo.user_code}</div>
            <p class="hint">Waiting for authorisation…</p>

        {:else if phase === 'done'}
            <p class="hint">Signed in! Redirecting…</p>
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
        max-width: 380px;
        background: var(--surface);
        border: 1px solid var(--border);
        border-radius: var(--radius-lg);
        padding: 2.5rem 2rem;
        text-align: center;
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 12px;
    }
    .heading {
        font-family: var(--font-display);
        font-size: 32px;
        font-weight: 700;
        color: var(--amber);
        margin: 0;
        letter-spacing: 1px;
    }
    .sub { font-size: 12px; color: var(--text-3); margin: 0; }

    .msg-error   { color: var(--red);    font-size: 13px; }
    .instruction { color: var(--text-2); font-size: 13px; margin: 0; }
    .hint        { color: var(--text-3); font-size: 12px; margin: 0; }

    .auth-link {
        color: var(--amber);
        font-size: 14px;
        font-weight: 500;
        text-decoration: none;
    }
    .auth-link:hover { text-decoration: underline; }

    .user-code {
        font-size: 2rem;
        font-weight: 700;
        letter-spacing: 0.25em;
        color: var(--text);
        font-family: monospace;
        background: var(--surface-2);
        padding: 8px 20px;
        border-radius: var(--radius);
        border: 1px solid var(--border);
    }
</style>
