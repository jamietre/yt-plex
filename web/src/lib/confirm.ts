import { writable } from 'svelte/store';

export interface ConfirmOptions {
	title: string;
	message: string;
	confirmLabel?: string;
	cancelLabel?: string;
}

interface ConfirmState extends ConfirmOptions {
	resolve: (value: boolean) => void;
}

export const confirmState = writable<ConfirmState | null>(null);

export function showConfirm(options: ConfirmOptions): Promise<boolean> {
	return new Promise((resolve) => {
		confirmState.set({ ...options, resolve });
	});
}
