// What a search engine or a link preview sees of the page: the head tags, the
// text that is there before any script runs, robots.txt and sitemap.xml.
//
// The texts come from the message catalogue, like everything else. The page
// is built to work from any address, so absolute URLs (canonical link, link
// preview picture, sitemap) exist only when the build is told where the page
// will live: SITE_URL=https://example.org/img2ico/ npm run build
import type { Plugin } from 'vite';
import { siApple, siGithub, siLinux } from 'simple-icons';
import { en } from './src/i18n/en.ts';

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

/** A page besides the converter: its own title, description and place below the site's address. */
export interface PageInfo {
  title: string;
  description: string;
  path: string;
}

/** The tags for <head> (after the charset and viewport ones). */
export function headTags(m: Messages, siteUrl: string, page?: PageInfo): string {
  const title = escapeHtml(page ? page.title : m['seo.title']);
  const description = escapeHtml(page ? page.description : m['seo.description']);
  const address = siteUrl + (page ? page.path : '');
  const tags = [
    `<title>${title}</title>`,
    `<meta name="description" content="${description}" />`,
    `<meta name="robots" content="index, follow, max-image-preview:large" />`,
    `<meta name="color-scheme" content="light dark" />`,
    // The theme the visitor chose, before the page is drawn (see src/theme.ts).
    `<script>try{var t=localStorage.getItem('img2ico.theme.v1');if(t==='light'||t==='dark')document.documentElement.dataset.theme=t}catch(e){}</script>`,
    `<meta name="theme-color" content="#16264a" media="(prefers-color-scheme: light)" />`,
    `<meta name="theme-color" content="#16264a" media="(prefers-color-scheme: dark)" />`,
    `<meta property="og:type" content="website" />`,
    `<meta property="og:site_name" content="${escapeHtml(m['app.name'])}" />`,
    `<meta property="og:title" content="${title}" />`,
    `<meta property="og:description" content="${description}" />`,
    `<meta property="og:locale" content="en_US" />`,
    `<meta name="twitter:card" content="${siteUrl ? 'summary_large_image' : 'summary'}" />`,
  ];
  if (siteUrl) {
    tags.push(
      `<link rel="canonical" href="${escapeHtml(address)}" />`,
      `<meta property="og:url" content="${escapeHtml(address)}" />`,
      `<meta property="og:image" content="${escapeHtml(siteUrl)}og-image.png" />`,
      `<meta property="og:image:width" content="1200" />`,
      `<meta property="og:image:height" content="630" />`,
      `<meta property="og:image:alt" content="${escapeHtml(m['app.name'] + ' – ' + m['app.tagline'])}" />`,
    );
  }
  if (!page) tags.push(`<script type="application/ld+json">\n${jsonForScript(structuredData(m, siteUrl))}\n</script>`);
  return tags.join('\n    ');
}

/** The heading of the page: there before the script runs, and what a crawler without scripts reads. */
export function headerHtml(m: Messages): string {
  return `<h1>${escapeHtml(m['hero.title'])}</h1>\n          <p class="lead">${escapeHtml(m['app.tagline'])}</p>`;
}

/** What the picture in the corner of the heading goes through: a picture of some type becomes an icon file. */
const SCENES = [
  { from: 'PNG', to: 'ICO' },
  { from: 'JPG', to: 'ICO' },
  { from: 'SVG', to: 'ICO' },
  { from: 'GIF', to: 'ICO' },
  { from: 'WEBP', to: 'ICNS' },
];

/**
 * The picture in the corner of the heading: one picture after another shrinks
 * into an icon and fans out into the sizes of the icon file, its label turning
 * from the picture type into ICO (or ICNS). Decoration only; the animation is
 * pure CSS, and without motion the first scene stands finished.
 */
export function artHtml(): string {
  const scenes = SCENES.map(
    (scene, i) => `<div class="scene art${i + 1}" style="--i:${i}">
            <div class="frame"><div class="pic"></div><span class="tag"><b class="from">${scene.from}</b><b class="to">${scene.to}</b></span></div>
            <div class="sizes"><i></i><i></i><i></i><i></i><i></i></div>
          </div>`,
  ).join('\n          ');
  return `<div class="hero-art" aria-hidden="true">
          ${scenes}
        </div>`;
}

// The logos of the systems (and GitHub): Apple, Tux and GitHub from Simple Icons (CC0), the four
// panes of Windows drawn here (Simple Icons has no Windows logo).
const WINDOWS_LOGO = 'M3 3h8.5v8.5H3zM12.5 3H21v8.5h-8.5zM3 12.5h8.5V21H3zM12.5 12.5H21V21h-8.5z';

function osIcon(path: string): string {
  return `<svg viewBox="0 0 24 24" width="18" height="18" fill="currentColor" aria-hidden="true"><path d="${path}"/></svg>`;
}

/** The help topics, in the order of the menu: the address below `help/` and the message key. */
export const HELP_TOPICS = [
  { slug: 'getting-started', key: 'start' },
  { slug: 'settings', key: 'settings' },
  { slug: 'file-types', key: 'types' },
  { slug: 'privacy', key: 'privacy' },
  { slug: 'keyboard-shortcuts', key: 'shortcuts' },
  { slug: 'command-line', key: 'cli' },
] as const;

export function helpPageInfo(m: Messages, slug: string): PageInfo | undefined {
  const topic = HELP_TOPICS.find((entry) => entry.slug === slug);
  if (!topic) return undefined;
  return {
    title: `${m[`help.${topic.key}.title`]} – ${m['app.name']}`,
    description: m[`help.${topic.key}.description`],
    path: `help/${slug}/`,
  };
}

/** The languages the page is planned in: the flag (drawn here, flag emoji do not show on Windows) and the name in that language. */
export const LANGUAGES = [
  { code: 'en', name: 'English', flag: '<rect width="24" height="16" fill="#012169"/><path d="M0 0l24 16M24 0 0 16" stroke="#fff" stroke-width="3.2"/><path d="M0 0l24 16M24 0 0 16" stroke="#c8102e" stroke-width="1.1"/><path d="M12 0v16M0 8h24" stroke="#fff" stroke-width="5.2"/><path d="M12 0v16M0 8h24" stroke="#c8102e" stroke-width="3"/>' },
  { code: 'de', name: 'Deutsch', flag: '<rect width="24" height="16" fill="#dd0000"/><rect width="24" height="5.34" fill="#000"/><rect y="10.66" width="24" height="5.34" fill="#ffce00"/>' },
  { code: 'es', name: 'Español', flag: '<rect width="24" height="16" fill="#aa151b"/><rect y="4" width="24" height="8" fill="#f1bf00"/>' },
  { code: 'pt-br', name: 'Português (Brasil)', flag: '<rect width="24" height="16" fill="#009c3b"/><path d="M12 1.6 22 8 12 14.4 2 8z" fill="#ffdf00"/><circle cx="12" cy="8" r="3.5" fill="#002776"/>' },
  { code: 'fr', name: 'Français', flag: '<rect width="24" height="16" fill="#fff"/><rect width="8" height="16" fill="#002654"/><rect x="16" width="8" height="16" fill="#ce1126"/>' },
] as const;

function flag(drawing: string): string {
  return `<svg class="flag" viewBox="0 0 24 16" width="22" height="15" aria-hidden="true">${drawing}</svg>`;
}

/** The half-filled circle of the light/dark switch. */
const THEME_ICON =
  '<svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><circle cx="12" cy="12" r="9"/><path d="M12 3v18a9 9 0 0 0 0-18z" fill="currentColor"/></svg>';

/**
 * The top bar's menus, from the left: help, the command-line tool (the latest release for each system:
 * GitHub redirects "latest" to the newest one), the language, the light/dark switch, GitHub. `prefix` leads
 * from the page to the site's root ("./" or "../../").
 */
export function navHtml(m: Messages, prefix = './'): string {
  const latest = `${REPOSITORY}/releases/latest/download/`;
  const item = (file: string, key: string, path: string) =>
    `<a href="${latest}${file}">${osIcon(path)}${escapeHtml(m[key])}</a>`;
  const help = HELP_TOPICS.map(
    (topic) => `<a href="${prefix}help/${topic.slug}/">${escapeHtml(m[`help.${topic.key}.title`])}</a>`,
  ).join('\n              ');
  const languages = LANGUAGES.map((language, i) =>
    i === 0
      ? `<a href="${prefix}" lang="${language.code}" aria-current="true">${flag(language.flag)}${escapeHtml(language.name)}</a>`
      : `<span class="soon" lang="${language.code}" aria-disabled="true" title="${escapeHtml(m['nav.soon'])}">${flag(language.flag)}${escapeHtml(language.name)}<small>${escapeHtml(m['nav.soon'])}</small></span>`,
  ).join('\n              ');
  return `<details class="menu">
            <summary>${escapeHtml(m['nav.help'])}</summary>
            <div class="menu-list">
              ${help}
            </div>
          </details>
          <details class="menu">
            <summary>${escapeHtml(m['nav.cli'])}</summary>
            <div class="menu-list">
              ${item('img2ico-windows.zip', 'nav.cli_windows', WINDOWS_LOGO)}
              ${item('img2ico-macos.zip', 'nav.cli_macos', siApple.path)}
              ${item('img2ico-linux.zip', 'nav.cli_linux', siLinux.path)}
              <a class="all" href="${REPOSITORY}/releases/latest">${escapeHtml(m['nav.cli_all'])}</a>
            </div>
          </details>
          <details class="menu lang">
            <summary aria-label="${escapeHtml(m['nav.language'])}">${flag(LANGUAGES[0].flag)}${escapeHtml(LANGUAGES[0].name)}</summary>
            <div class="menu-list">
              ${languages}
            </div>
          </details>
          <button type="button" class="theme-switch" disabled title="${escapeHtml(m['nav.theme'])}" aria-label="${escapeHtml(m['nav.theme'])}">${THEME_ICON}</button>
          <a href="${REPOSITORY}">${osIcon(siGithub.path)}${escapeHtml(m['nav.github'])}</a>`;
}

/** A help page: the list of topics on the left, the page on the right. */
export function helpHtml(m: Messages, slug: string, prefix: string): string {
  const e = (key: string) => escapeHtml(m[key]);
  const links = HELP_TOPICS.map(
    (topic) =>
      `<li><a href="${prefix}help/${topic.slug}/"${topic.slug === slug ? ' aria-current="page"' : ''}>${e(`help.${topic.key}.title`)}</a></li>`,
  ).join('\n            ');
  const topic = HELP_TOPICS.find((entry) => entry.slug === slug);
  const key = topic ? topic.key : 'start';
  return `<div class="help">
        <nav class="help-nav" aria-label="${e('help.topics')}">
          <ul>
            ${links}
          </ul>
        </nav>
        <article class="help-page">
          <h1>${e(`help.${key}.title`)}</h1>
          <p class="lead">${e(`help.${key}.description`)}</p>
          <p class="placeholder">${e('help.placeholder')}</p>
        </article>
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
  const addresses = [siteUrl, ...HELP_TOPICS.map((topic) => `${siteUrl}help/${topic.slug}/`)];
  const urls = addresses.map((address) => `  <url><loc>${escapeHtml(address)}</loc></url>`).join('\n');
  return `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${urls}\n</urlset>\n`;
}

/** Fills the placeholders of index.html and writes robots.txt (and sitemap.xml when the address is known). */
export function seo(address?: string): Plugin {
  const siteUrl = normalizeSiteUrl(address);
  return {
    name: 'img2ico-seo',
    transformIndexHtml(html, context) {
      // A page below help/<topic>/ is two folders deep; the converter is at the root.
      const helpSlug = /\/help\/([^/]+)\//.exec(context.path)?.[1];
      const page = helpSlug ? helpPageInfo(en, helpSlug) : undefined;
      const prefix = helpSlug ? '../../' : './';
      return html
        .replace('<!--seo:head-->', () => headTags(en, siteUrl, page))
        .replace('<!--seo:help-->', () => (helpSlug ? helpHtml(en, helpSlug, prefix) : ''))
        .replace('<!--seo:header-->', () => headerHtml(en))
        .replace('<!--seo:art-->', () => artHtml())
        .replace('<!--seo:nav-->', () => navHtml(en, prefix))
        .replace('<!--seo:name-->', () => escapeHtml(en['app.name']))
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
