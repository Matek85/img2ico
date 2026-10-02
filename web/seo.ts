// What a search engine or a link preview sees of the page: the head tags, the
// text that is there before any script runs, robots.txt and sitemap.xml.
//
// The texts come from the message catalogue, like everything else. The page
// is built to work from any address, so absolute URLs (canonical link, link
// preview picture, sitemap) exist only when the build is told where the page
// will live: SITE_URL=https://example.org/img2ico/ npm run build
import type { Plugin } from 'vite';
import { en } from './src/i18n/en';

type Messages = Record<string, string>;

const REPOSITORY = 'https://github.com/Matek85/img2ico';

export function escapeHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

/** The address with one trailing slash, or '' when there is none (or it is not http/https). */
export function normalizeSiteUrl(value: string | undefined): string {
  const text = (value ?? '').trim();
  if (!/^https?:\/\/[^\s]+$/i.test(text)) return '';
  return text.replace(/\/+$/, '') + '/';
}

/** JSON for a <script> element: nothing in it can end the element early. */
export function jsonForScript(value: unknown): string {
  return JSON.stringify(value, null, 2).replace(/</g, '\\u003c');
}

export function structuredData(m: Messages, siteUrl: string): object {
  return {
    '@context': 'https://schema.org',
    '@type': 'WebApplication',
    name: m['app.name'],
    description: m['seo.description'],
    ...(siteUrl ? { url: siteUrl, image: siteUrl + 'og-image.png' } : {}),
    applicationCategory: 'MultimediaApplication',
    operatingSystem: 'Any',
    browserRequirements: 'Requires JavaScript and WebAssembly',
    inLanguage: 'en',
    isAccessibleForFree: true,
    offers: { '@type': 'Offer', price: '0', priceCurrency: 'USD' },
    sameAs: [REPOSITORY],
  };
}

/** The tags for <head> (after the charset and viewport ones). */
export function headTags(m: Messages, siteUrl: string): string {
  const title = escapeHtml(m['seo.title']);
  const description = escapeHtml(m['seo.description']);
  const tags = [
    `<title>${title}</title>`,
    `<meta name="description" content="${description}" />`,
    `<meta name="robots" content="index, follow, max-image-preview:large" />`,
    `<meta name="color-scheme" content="light dark" />`,
    `<meta name="theme-color" content="#f7f7f9" media="(prefers-color-scheme: light)" />`,
    `<meta name="theme-color" content="#14141a" media="(prefers-color-scheme: dark)" />`,
    `<meta property="og:type" content="website" />`,
    `<meta property="og:site_name" content="${escapeHtml(m['app.name'])}" />`,
    `<meta property="og:title" content="${title}" />`,
    `<meta property="og:description" content="${description}" />`,
    `<meta property="og:locale" content="en_US" />`,
    `<meta name="twitter:card" content="${siteUrl ? 'summary_large_image' : 'summary'}" />`,
  ];
  if (siteUrl) {
    tags.push(
      `<link rel="canonical" href="${escapeHtml(siteUrl)}" />`,
      `<meta property="og:url" content="${escapeHtml(siteUrl)}" />`,
      `<meta property="og:image" content="${escapeHtml(siteUrl)}og-image.png" />`,
      `<meta property="og:image:width" content="1200" />`,
      `<meta property="og:image:height" content="630" />`,
      `<meta property="og:image:alt" content="${escapeHtml(m['app.name'] + ' – ' + m['app.tagline'])}" />`,
    );
  }
  tags.push(`<script type="application/ld+json">\n${jsonForScript(structuredData(m, siteUrl))}\n</script>`);
  return tags.join('\n    ');
}

/** The heading of the page: there before the script runs, and what a crawler without scripts reads. */
export function headerHtml(m: Messages): string {
  return `<h1>${escapeHtml(m['hero.title'])}</h1>\n          <p class="lead">${escapeHtml(m['app.tagline'])}</p>`;
}

const FILE_ICON =
  '<svg viewBox="0 0 24 24" width="36" height="36" fill="currentColor"><path d="M6 2h8l5 5v13a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2zm7 1.5V8h4.5L13 3.5zM8 18h8l-2.5-3.5-2 2.5-1.2-1.5L8 18z"/></svg>';

/** The picture in the corner of the heading: a file turning into another. Decoration only. */
export function artHtml(m: Messages): string {
  return `<div class="hero-art" aria-hidden="true">
          <div class="tile">${FILE_ICON}<b>PNG</b></div>
          <div class="join"><span class="spin"><svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 12a8 8 0 0 1 13.5-5.8L20 8.5M20 4v4.5h-4.5M20 12a8 8 0 0 1-13.5 5.8L4 15.5M4 20v-4.5h4.5"/></svg></span><small>${escapeHtml(m['hero.to'])}</small></div>
          <div class="tile out">${FILE_ICON}<b>ICO</b></div>
        </div>`;
}

/** The text under the converter. */
export function aboutHtml(m: Messages): string {
  const e = (key: string) => escapeHtml(m[key]);
  const list = (tag: 'ul' | 'ol', prefix: string, count: number) =>
    `<${tag}>\n` +
    Array.from({ length: count }, (_, i) => `          <li>${e(`${prefix}${i + 1}`)}</li>`).join('\n') +
    `\n        </${tag}>`;
  const questions = ['free', 'private', 'sizes', 'favicon', 'formats', 'cli']
    .map((id) => `<div class="qa"><h3>${e(`about.q_${id}`)}</h3>\n          <p>${e(`about.a_${id}`)}</p></div>`)
    .join('\n        ');
  return `<h2>${e('about.title')}</h2>
        <p>${e('about.intro')}</p>
        <h2>${e('about.how_title')}</h2>
        ${list('ol', 'about.how_', 3)}
        <h2>${e('about.can_title')}</h2>
        ${list('ul', 'about.can_', 5)}
        <h2>${e('about.faq_title')}</h2>
        <div class="qas">
        ${questions}
        </div>
        <p><a href="${REPOSITORY}">${e('about.source')}</a></p>`;
}

export function robotsTxt(siteUrl: string): string {
  return `User-agent: *\nAllow: /\n${siteUrl ? `\nSitemap: ${siteUrl}sitemap.xml\n` : ''}`;
}

export function sitemapXml(siteUrl: string): string {
  return `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n  <url><loc>${escapeHtml(siteUrl)}</loc></url>\n</urlset>\n`;
}

/** Fills the placeholders of index.html and writes robots.txt (and sitemap.xml when the address is known). */
export function seo(address?: string): Plugin {
  const siteUrl = normalizeSiteUrl(address);
  return {
    name: 'img2ico-seo',
    transformIndexHtml(html) {
      return html
        .replace('<!--seo:head-->', () => headTags(en, siteUrl))
        .replace('<!--seo:header-->', () => headerHtml(en))
        .replace('<!--seo:art-->', () => artHtml(en))
        .replace('<!--seo:name-->', () => escapeHtml(en['app.name']))
        .replace('<!--seo:github-->', () => escapeHtml(en['nav.github']))
        .replace('<!--seo:repository-->', () => REPOSITORY)
        .replace('<!--seo:noscript-->', () => escapeHtml(en['seo.noscript']))
        .replace('<!--seo:privacy-->', () => escapeHtml(en['app.privacy']))
        .replace('<!--seo:about-->', () => aboutHtml(en));
    },
    generateBundle() {
      this.emitFile({ type: 'asset', fileName: 'robots.txt', source: robotsTxt(siteUrl) });
      if (siteUrl) this.emitFile({ type: 'asset', fileName: 'sitemap.xml', source: sitemapXml(siteUrl) });
    },
  };
}
