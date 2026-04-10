import { writable } from 'svelte/store';

export interface NavSearchConfig {
    value: string;
    placeholder: string;
    onInput: (value: string) => void;
}

/** Set by a page to show a search box in the top nav. Cleared on page destroy. */
export const navSearch = writable<NavSearchConfig | null>(null);
