import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

// Dev server proxies API + WebSocket to a running vanifold-core.
// Point at the Pi with: VANIFOLD_API=http://vanhub.local:8480 npm run dev
const apiTarget = process.env.VANIFOLD_API ?? 'http://localhost:8480';

export default defineConfig({
	server: {
		proxy: {
			'/api': { target: apiTarget, ws: true, changeOrigin: true }
		}
	},
	plugins: [
		sveltekit({
			compilerOptions: {
				// Force runes mode for the project, except for libraries. Can be removed in svelte 6.
				runes: ({ filename }) =>
					filename.split(/[/\\]/).includes('node_modules') ? undefined : true
			},

			// SPA build: static assets get embedded into the vanifold-core binary,
			// which serves index.html for all routes. No Node runtime on the hub.
			adapter: adapter({ fallback: 'index.html' })
		})
	]
});
