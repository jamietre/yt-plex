import { writable } from 'svelte/store';
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

export function createWsStore() {
    const { subscribe, update } = writable<WsMessage | null>(null);

    let socket: WebSocket | null = null;

    function connect() {
        const proto = location.protocol === 'https:' ? 'wss' : 'ws';
        socket = new WebSocket(`${proto}://${location.host}/ws`);
        socket.onmessage = (ev) => {
            try {
                const msg: WsMessage = JSON.parse(ev.data);
                update(() => msg);
            } catch {
                /* ignore malformed */
            }
        };
        socket.onclose = () => {
            // Reconnect after 3s
            setTimeout(connect, 3000);
        };
    }

    function disconnect() {
        socket?.close();
        socket = null;
    }

    return { subscribe, connect, disconnect };
}
