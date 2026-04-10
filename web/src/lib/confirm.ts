import { writable } from 'svelte/store';

export interface ConfirmOptions {
	title: string;
	message: string;
	confirmLabel?: string;
	cancelLabel?: string;
}

export const confirmState = writable<ConfirmOptions | null>(null);

let _resolve: ((value: boolean) => void) | null = null;

export function showConfirm(options: ConfirmOptions): Promise<boolean> {
	return new Promise((resolve) => {
		_resolve = resolve;
		confirmState.set(options);
	});
}

export function resolveConfirm(value: boolean) {
	_resolve?.(value);
	_resolve = null;
	confirmState.set(null);
}
