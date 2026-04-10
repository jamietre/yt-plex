<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { page } from '$app/stores';
    import { goto } from '$app/navigation';
    import { getAuthFlow, startDeviceLogin, pollDeviceAuth, type AuthFlow, type DeviceLoginResponse } from '$lib/api';

    const errorMessages: Record<string, string> = {
        denied:  'Access denied — your account is not authorised.',
        invalid: 'Sign-in failed — invalid request. Please try again.',
        server:  'A server error occurred. Please try again.',
    };

    const errorKey = $derived($page.url?.searchParams.get('error') ?? '');
    const errorMsg = $derived(errorMessages[errorKey] ?? '');

    let flow = $state<AuthFlow>('authorization_code');
    let device = $state<DeviceLoginResponse | null>(null);
    let pollStatus = $state<'idle' | 'pending' | 'expired'>('idle');
    let pollTimer: ReturnType<typeof setInterval> | null = null;

    onMount(async () => {
        flow = await getAuthFlow();
        if (flow === 'device') {
            await startDevice();
        }
    });

    onDestroy(() => {
        if (pollTimer) clearInterval(pollTimer);
    });

    async function startDevice() {
        try {
            device = await startDeviceLogin();
            pollStatus = 'pending';
            const interval = Math.max((device.interval ?? 5) * 1000, 5000);
            pollTimer = setInterval(async () => {
                if (!device) return;
                const status = await pollDeviceAuth(device.poll_token);
                if (status === 'done') {
                    clearInterval(pollTimer!);
                    goto('/');
                } else if (status === 'expired') {
                    clearInterval(pollTimer!);
                    pollStatus = 'expired';
                    device = null;
                }
            }, interval);
        } catch {
            pollStatus = 'expired';
        }
    }
</script>

<div class="page">
    <div class="card">
        <h1 class="heading">yt-plex</h1>
        <p class="sub">Admin sign-in</p>

        {#if errorMsg}
            <p class="msg-error">{errorMsg}</p>
        {/if}

        {#if flow === 'authorization_code'}
            <a href="/api/auth/login?return_to={encodeURIComponent(window.location.origin)}" class="btn-google">Sign in with Google</a>

        {:else if flow === 'device'}
            {#if pollStatus === 'expired' || (pollStatus === 'idle' && !device)}
                <p class="msg-error">Code expired or unavailable.</p>
                <button class="btn-google" onclick={startDevice}>Try again</button>

            {:else if device}
                <p class="device-instructions">
                    Visit <a href={device.verification_url} target="_blank" rel="noopener">{device.verification_url}</a>
                    and enter this code:
                </p>
                <div class="device-code">{device.user_code}</div>
                <p class="device-waiting">Waiting for sign-in…</p>
            {:else}
                <p class="device-waiting">Loading…</p>
            {/if}
        {/if}
    </div>
</div>

<style>
    .page {
        min-height: calc(100vh / var(--zoom, 1));
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
        gap: 16px;
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
    .msg-error { color: var(--red); font-size: 13px; margin: 0; }

    .btn-google {
        display: inline-block;
        background: var(--amber);
        color: #000;
        font-size: 13px;
        font-weight: 700;
        font-family: var(--font-ui);
        padding: 9px 20px;
        border-radius: var(--radius);
        text-decoration: none;
        border: none;
        cursor: pointer;
        transition: background 0.12s;
    }
    .btn-google:hover { background: var(--amber-glow); }

    .device-instructions {
        font-size: 13px;
        color: var(--text-2);
        margin: 0;
        line-height: 1.6;
    }
    .device-instructions a { color: var(--amber); }

    .device-code {
        font-family: var(--font-display);
        font-size: 28px;
        font-weight: 700;
        letter-spacing: 6px;
        color: var(--text);
        background: var(--surface-2);
        border: 1px solid var(--border);
        border-radius: var(--radius);
        padding: 12px 24px;
    }

    .device-waiting {
        font-size: 12px;
        color: var(--text-3);
        margin: 0;
    }
</style>
