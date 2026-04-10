<script lang="ts">
    import { onMount } from 'svelte';
    import { loadPrefs, savePrefs, applyPrefs, FONT_SIZES, type Prefs } from '$lib/prefs';

    let prefs = $state<Prefs>({ fontSize: 14, theme: 'dark' });

    onMount(() => {
        prefs = loadPrefs();
    });

    function setTheme(theme: Prefs['theme']) {
        prefs = { ...prefs, theme };
        savePrefs(prefs);
        applyPrefs(prefs);
    }

    function setFontSize(size: number) {
        prefs = { ...prefs, fontSize: size };
        savePrefs(prefs);
        applyPrefs(prefs);
    }

    function adjustFont(dir: 1 | -1) {
        const idx = FONT_SIZES.indexOf(prefs.fontSize);
        const next = FONT_SIZES[idx + dir];
        if (next !== undefined) setFontSize(next);
    }
</script>

<div class="page">
    <h1 class="heading">Settings</h1>

    <section class="section">
        <h2 class="section-title">Appearance</h2>

        <div class="row">
            <span class="label">Theme</span>
            <div class="btn-group">
                <button
                    class="opt-btn"
                    class:active={prefs.theme === 'dark'}
                    onclick={() => setTheme('dark')}
                >Dark</button>
                <button
                    class="opt-btn"
                    class:active={prefs.theme === 'light'}
                    onclick={() => setTheme('light')}
                >Light</button>
            </div>
        </div>

        <div class="row">
            <span class="label">Font size</span>
            <div class="font-controls">
                <button
                    class="size-btn"
                    onclick={() => adjustFont(-1)}
                    disabled={prefs.fontSize === FONT_SIZES[0]}
                    aria-label="Decrease font size"
                >A−</button>
                <span class="size-value">{prefs.fontSize}px</span>
                <button
                    class="size-btn"
                    onclick={() => adjustFont(1)}
                    disabled={prefs.fontSize === FONT_SIZES[FONT_SIZES.length - 1]}
                    aria-label="Increase font size"
                >A+</button>
            </div>
        </div>
    </section>
</div>

<style>
    .page {
        max-width: 480px;
        padding: 28px 24px;
    }

    .heading {
        font-family: var(--font-display);
        font-size: 22px;
        font-weight: 700;
        color: var(--text);
        margin: 0 0 28px;
    }

    .section-title {
        font-size: 11px;
        font-weight: 700;
        text-transform: uppercase;
        letter-spacing: 0.6px;
        color: var(--text-3);
        margin: 0 0 14px;
    }

    .section {
        background: var(--surface);
        border: 1px solid var(--border);
        border-radius: var(--radius-lg);
        padding: 16px 20px;
        display: flex;
        flex-direction: column;
        gap: 16px;
    }

    .row {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 16px;
    }

    .label {
        font-size: 13px;
        color: var(--text);
    }

    .btn-group {
        display: flex;
        border: 1px solid var(--border-2);
        border-radius: var(--radius);
        overflow: hidden;
    }

    .opt-btn {
        background: transparent;
        border: none;
        color: var(--text-2);
        font-family: var(--font-ui);
        font-size: 12px;
        font-weight: 500;
        padding: 5px 14px;
        cursor: pointer;
        transition: background 0.1s, color 0.1s;
    }
    .opt-btn + .opt-btn { border-left: 1px solid var(--border-2); }
    .opt-btn:hover:not(.active) { background: var(--surface-2); color: var(--text); }
    .opt-btn.active { background: var(--amber); color: #000; font-weight: 600; }

    .font-controls {
        display: flex;
        align-items: center;
        gap: 10px;
    }

    .size-btn {
        background: var(--surface-2);
        border: 1px solid var(--border-2);
        color: var(--text-2);
        font-family: var(--font-ui);
        font-size: 12px;
        font-weight: 600;
        padding: 4px 10px;
        border-radius: var(--radius);
        cursor: pointer;
        transition: border-color 0.15s, color 0.15s;
    }
    .size-btn:hover:not(:disabled) { border-color: var(--amber); color: var(--amber); }
    .size-btn:disabled { opacity: 0.35; cursor: default; }

    .size-value {
        font-size: 13px;
        color: var(--text);
        min-width: 36px;
        text-align: center;
    }
</style>
