import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { seo } from './seo.ts';

// Only the environment is read from Node here.
declare const process: { env: Record<string, string | undefined> };

// base './' makes every URL in the build relative, so the finished dist/
// folder works from any address: github.io/img2ico/, a domain of its own, or
// a sub-folder of some other web space. The few things that need a full
// address (canonical link, link preview picture, sitemap) are written only
// when the build is told where the page will live, see seo.ts:
//   SITE_URL=https://example.org/img2ico/ npm run build
export default defineConfig({
  base: './',
  plugins: [svelte(), seo(process.env.SITE_URL)],
  worker: { format: 'es' },
  build: { target: 'es2022' },
  test: { environment: 'node' },
});
