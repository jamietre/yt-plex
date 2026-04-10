export type Theme = 'dark' | 'light';

export interface Prefs {
    fontSize: number;
    theme: Theme;
}

export const FONT_SIZES = [11, 12, 13, 14, 15, 16, 18];
const DEFAULTS: Prefs = { fontSize: 14, theme: 'dark' };
const KEY = 'yt_plex_prefs';

export function loadPrefs(): Prefs {
    if (typeof localStorage === 'undefined') return { ...DEFAULTS };
    try {
        const raw = localStorage.getItem(KEY);
        if (!raw) return { ...DEFAULTS };
        const parsed = JSON.parse(raw);
        return {
            fontSize: FONT_SIZES.includes(parsed.fontSize) ? parsed.fontSize : DEFAULTS.fontSize,
            theme: parsed.theme === 'light' ? 'light' : 'dark',
        };
    } catch {
        return { ...DEFAULTS };
    }
}

export function savePrefs(prefs: Prefs): void {
    localStorage.setItem(KEY, JSON.stringify(prefs));
}

export function applyPrefs(prefs: Prefs): void {
    const zoom = prefs.fontSize / 14;
    document.documentElement.style.zoom = String(zoom);
    // Pages that use 100vh for centering must compensate: min-height: calc(100vh / var(--zoom, 1))
    document.documentElement.style.setProperty('--zoom', String(zoom));
    document.documentElement.setAttribute('data-theme', prefs.theme);
}
