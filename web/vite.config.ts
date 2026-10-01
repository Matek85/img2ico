import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';

// base './' makes every URL in the build relative, so the finished dist/
// folder works from any address: github.io/img2ico/, a domain of its own, or
// a sub-folder of some other web space.
export default defineConfig({
  base: './',
  plugins: [svelte()],
  worker: { format: 'es' },
  build: { target: 'es2022' },
  test: { environment: 'node' },
});

