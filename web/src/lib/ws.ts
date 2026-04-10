import { writable, derived } from 'svelte/store';
import type { Job } from './api';

export interface WsMessage {
    job_id: string;
    status: Job['status'];
    channel_name: string | null;
    title: string | null;
    error: string | null;
    progress?: number | null;
    youtube_id?: string | null;
}

// ── Singleton shared connection ──────────────────────────────────────────────
// One WebSocket for the whole app lifetime. Components subscribe to the store;
// they don't manage the socket themselves.

const { subscribe, set } = writable<WsMessage | null>(null);

let socket: WebSocket | null = null;
let intentionalClose = false;
let retryTimer: ReturnType<typeof setTimeout> | null = null;

function connect() {
    // Guard against all non-closed states (CONNECTING=0, OPEN=1, CLOSING=2).
    // If we replaced a CLOSING socket without this guard, its onclose would fire
    // later with intentionalClose=false and schedule a spurious retry.
    if (socket && socket.readyState !== WebSocket.CLOSED) return;

    intentionalClose = false;

    // Detach handlers from any dead CLOSED socket before replacing it, so a
    // stale onclose can't schedule a retry after we've already moved on.
    if (socket) {
        socket.onclose = null;
        socket.onerror = null;
        socket.onmessage = null;
    }

    const proto = location.protocol === 'https:' ? 'wss' : 'ws';
    socket = new WebSocket(`${proto}://${location.host}/ws`);
    socket.onmessage = (ev) => {
        try { set(JSON.parse(ev.data) as WsMessage); } catch { /* ignore */ }
    };
    socket.onclose = () => {
        if (!intentionalClose) {
            retryTimer = setTimeout(connect, 3000);
        }
    };
    socket.onerror = () => {
        // onclose will fire after onerror; let it handle retry
    };
}

function disconnect() {
    intentionalClose = true;
    if (retryTimer) { clearTimeout(retryTimer); retryTimer = null; }
    if (socket) { socket.close(); socket = null; }
}

/** The shared WS message store. Call `wsConnect()` once from the layout. */
export const wsMessages = { subscribe };
export const wsConnect = connect;
export const wsDisconnect = disconnect;

// ── Legacy per-page store factory (kept for gradual migration) ───────────────
/** @deprecated Use `wsMessages` singleton instead */
export function createWsStore() {
    return {
        subscribe,
        connect,
        disconnect,
    };
}
