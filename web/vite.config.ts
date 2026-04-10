import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [sveltekit()],
	build: {
		sourcemap: true,
	},
	server: {
		proxy: {
			'/api': 'http://127.0.0.1:32113',
			'/ws': { target: 'ws://127.0.0.1:32113', ws: true }
		}
	}
});
