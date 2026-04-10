// web/src/lib/toast.ts
import { writable } from 'svelte/store';

export type ToastVariant = 'success' | 'error' | 'info';

export interface Toast {
    id: number;
    message: string;
    variant: ToastVariant;
}

export const toasts = writable<Toast[]>([]);

let nextId = 0;

export function toast(message: string, options?: { variant?: ToastVariant; duration?: number }) {
    const id = nextId++;
    const variant = options?.variant ?? 'success';
    const duration = options?.duration ?? 3500;

    toasts.update(ts => [...ts, { id, message, variant }]);

    setTimeout(() => dismissToast(id), duration);
}

export function dismissToast(id: number) {
    toasts.update(ts => ts.filter(t => t.id !== id));
}
