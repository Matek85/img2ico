import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { languagePages, TEMPLATES, writeLanguagePages } from './pages.ts';
import { seo } from './seo.ts';

// Only the environment is read from Node here.
declare const process: { env: Record<string, string | undefined> };

// The pages of the other languages are copies of the templates (see pages.ts).
writeLanguagePages(new URL('.', import.meta.url));

// base './' makes every URL in the build relative, so the finished dist/
// folder works from any address: github.io/img2ico/, a domain of its own, or
// a sub-folder of some other web space. The few things that need a full
// address (canonical link, link preview picture, sitemap) are written only
// when the build is told where the page will live, see seo.ts:
//   SITE_URL=https://example.org/img2ico/ npm run build
// The footer says when the page was built and from which commit (CI knows it; a local build says "dev").
const BUILD = {
  date: new Date().toISOString().slice(0, 10),
  commit: process.env.GITHUB_SHA?.slice(0, 7) ?? 'dev',
};

export default defineConfig({
  base: './',
  define: {
    __BUILD_DATE__: JSON.stringify(BUILD.date),
    __BUILD_COMMIT__: JSON.stringify(BUILD.commit),
  },
  plugins: [svelte(), seo(process.env.SITE_URL)],
  worker: { format: 'es' },
  // The converter and the help pages are pages of their own (each a folder with an index.html).
  build: {
    target: 'es2022',
    rollupOptions: {
      input: [...TEMPLATES, ...languagePages()],
    },
  },
  test: { environment: 'node' },
});
