// What a search engine or a link preview sees of the page: the head tags, the
// text that is there before any script runs, robots.txt and sitemap.xml.
//
// The texts come from the message catalogue of the page's language, like everything else.
// English is at the root of the site, every other language in a folder of its own
// (de/, es/, pt-br/, fr/) with the same pages. The page
// is built to work from any address, so absolute URLs (canonical link, link
// preview picture, sitemap) exist only when the build is told where the page
// will live: SITE_URL=https://example.org/img2ico/ npm run build
import type { Plugin } from 'vite';
import { siApple, siGithub, siLinux } from 'simple-icons';
import { CATALOGUES } from './catalogues.ts';
import { escapeHtml, helpBodyHtml, inlineHtml } from './helpContent.ts';

type Messages = Record<string, string>;

const REPOSITORY = 'https://github.com/Matek85/img2ico';

export { escapeHtml };

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

export function structuredData(m: Messages, siteUrl: string, lang = 'en'): object {
  return {
    '@context': 'https://schema.org',
    '@type': 'WebApplication',
    name: m['app.name'],
    description: m['seo.description'],
    ...(siteUrl ? { url: siteUrl, image: siteUrl + 'og-image.png' } : {}),
    applicationCategory: 'MultimediaApplication',
    operatingSystem: 'Any',
    browserRequirements: 'Requires JavaScript and WebAssembly',
    inLanguage: lang,
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

/**
 * Sends a visitor of an English page to the page in their language: the one they chose before, or the
 * first language of the browser's list that the site has (English first in that list: stay). Only the
 * English pages have it; the choice is made with the language menu (see src/menus.ts).
 */
export function languageRedirect(prefix: string, pagePath: string): string {
  const others = LANGUAGES.filter((language) => language.code !== 'en');
  const map = Object.fromEntries(others.map((language) => [language.code.split('-')[0], language.code]));
  const codes = others.map((language) => language.code).join('|');
  return `<script>try{var s=localStorage.getItem('img2ico.lang.v1'),M=${JSON.stringify(map)},l=null;if(s){l=s}else{var a=navigator.languages||[navigator.language];for(var i=0;i<a.length;i++){var p=String(a[i]).toLowerCase().split('-')[0];if(p==='en')break;if(M[p]){l=M[p];break}}}if(l&&/^(${codes})$/.test(l))location.replace('${prefix}'+l+'/${pagePath}')}catch(e){}</script>`;
}

/** The tags for <head> (after the charset and viewport ones). `prefix` leads from the page to the site's root. */
export function headTags(m: Messages, siteUrl: string, page?: PageInfo, lang: string = 'en', prefix = './'): string {
  const title = escapeHtml(page ? page.title : m['seo.title']);
  const description = escapeHtml(page ? page.description : m['seo.description']);
  const pagePath = page ? page.path : '';
  const address = siteUrl + langDir(lang) + pagePath;
  const language = LANGUAGES.find((entry) => entry.code === lang) ?? LANGUAGES[0];
  const tags = [
    `<title>${title}</title>`,
    `<meta name="description" content="${description}" />`,
    `<meta name="robots" content="index, follow, max-image-preview:large" />`,
    `<meta name="color-scheme" content="light dark" />`,
    `<link rel="icon" href="${prefix}favicon.ico" sizes="48x48" />`,
    `<link rel="icon" type="image/png" href="${prefix}favicon.png" />`,
    // The theme the visitor chose, before the page is drawn (see src/theme.ts).
    `<script>try{var t=localStorage.getItem('img2ico.theme.v1');if(t==='light'||t==='dark')document.documentElement.dataset.theme=t}catch(e){}</script>`,
    `<meta name="theme-color" content="#16264a" media="(prefers-color-scheme: light)" />`,
    `<meta name="theme-color" content="#16264a" media="(prefers-color-scheme: dark)" />`,
    `<meta property="og:type" content="website" />`,
    `<meta property="og:site_name" content="${escapeHtml(m['app.name'])}" />`,
    `<meta property="og:title" content="${title}" />`,
    `<meta property="og:description" content="${description}" />`,
    `<meta property="og:locale" content="${language.locale}" />`,
    `<meta name="twitter:card" content="${siteUrl ? 'summary_large_image' : 'summary'}" />`,
  ];
  if (siteUrl) {
    tags.push(
      `<link rel="canonical" href="${escapeHtml(address)}" />`,
      `<meta property="og:url" content="${escapeHtml(address)}" />`,
      ...LANGUAGES.map((entry) => `<link rel="alternate" hreflang="${langTag(entry.code)}" href="${escapeHtml(siteUrl + langDir(entry.code) + pagePath)}" />`),
      `<link rel="alternate" hreflang="x-default" href="${escapeHtml(siteUrl + pagePath)}" />`,
      `<meta property="og:image" content="${escapeHtml(siteUrl)}og-image.png" />`,
      `<meta property="og:image:width" content="1200" />`,
      `<meta property="og:image:height" content="630" />`,
      `<meta property="og:image:alt" content="${escapeHtml(m['app.name'] + ' – ' + m['app.tagline'])}" />`,
    );
  }
  if (lang === 'en') tags.push(languageRedirect(prefix, pagePath));
  if (!page) tags.push(`<script type="application/ld+json">\n${jsonForScript(structuredData(m, siteUrl, lang))}\n</script>`);
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

/**
 * The languages of the page: the flag (drawn here, flag emoji do not show on Windows), the name in that language, the
 * locale for link previews, and whether a notice says that an AI translated the page (until a person has read it).
 */
export const LANGUAGES = [
  { code: 'en', locale: 'en_US', ai: false, name: 'English', flag: '<rect width="24" height="16" fill="#012169"/><path d="M0 0l24 16M24 0 0 16" stroke="#fff" stroke-width="3.2"/><path d="M0 0l24 16M24 0 0 16" stroke="#c8102e" stroke-width="1.1"/><path d="M12 0v16M0 8h24" stroke="#fff" stroke-width="5.2"/><path d="M12 0v16M0 8h24" stroke="#c8102e" stroke-width="3"/>' },
  { code: 'de', locale: 'de_DE', ai: false, name: 'Deutsch', flag: '<rect width="24" height="16" fill="#dd0000"/><rect width="24" height="5.34" fill="#000"/><rect y="10.66" width="24" height="5.34" fill="#ffce00"/>' },
  { code: 'es', locale: 'es_ES', ai: true, name: 'Español', flag: '<rect width="24" height="16" fill="#aa151b"/><rect y="4" width="24" height="8" fill="#f1bf00"/>' },
  { code: 'pt-br', locale: 'pt_BR', ai: true, name: 'Português (Brasil)', flag: '<rect width="24" height="16" fill="#009c3b"/><path d="M12 1.6 22 8 12 14.4 2 8z" fill="#ffdf00"/><circle cx="12" cy="8" r="3.5" fill="#002776"/>' },
  { code: 'fr', locale: 'fr_FR', ai: true, name: 'Français', flag: '<rect width="24" height="16" fill="#fff"/><rect width="8" height="16" fill="#002654"/><rect x="16" width="8" height="16" fill="#ce1126"/>' },
] as const;

export type LanguageCode = (typeof LANGUAGES)[number]['code'];

/** The language as a tag for `lang` and `hreflang`: "pt-br" is written "pt-BR". */
export function langTag(code: string): string {
  return code
    .split('-')
    .map((part, i) => (i ? part.toUpperCase() : part))
    .join('-');
}

/** The folder of a language below the site's root: none for English. */
export function langDir(code: string): string {
  return code === 'en' ? '' : `${code}/`;
}

function flag(drawing: string): string {
  return `<svg class="flag" viewBox="0 0 24 16" width="22" height="15" aria-hidden="true">${drawing}</svg>`;
}

/** The half-filled circle of the light/dark switch. */
const THEME_ICON =
  '<svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><circle cx="12" cy="12" r="9"/><path d="M12 3v18a9 9 0 0 0 0-18z" fill="currentColor"/></svg>';

/**
 * The top bar's menus, from the left: help, the command-line tool (the latest release for each system:
 * GitHub redirects "latest" to the newest one), GitHub; then, set off, the language and the light/dark switch. `prefix` leads
 * from the page to the site's root ("./", "../", "../../" or "../../../"), `lang` is the language of the page and
 * `pagePath` the page below the language's folder ("" or "help/<topic>/"), so each language leads to the same page.
 */
export function navHtml(m: Messages, prefix = './', lang: string = 'en', pagePath = ''): string {
  const latest = `${REPOSITORY}/releases/latest/download/`;
  const item = (file: string, key: string, path: string) =>
    `<a href="${latest}${file}">${osIcon(path)}${escapeHtml(m[key])}</a>`;
  const help = HELP_TOPICS.map(
    (topic) => `<a href="${prefix}${langDir(lang)}help/${topic.slug}/">${escapeHtml(m[`help.${topic.key}.title`])}</a>`,
  ).join('\n              ');
  const current = LANGUAGES.find((language) => language.code === lang) ?? LANGUAGES[0];
  const languages = LANGUAGES.map(
    (language) =>
      `<a href="${prefix}${langDir(language.code)}${pagePath}" lang="${langTag(language.code)}" hreflang="${langTag(language.code)}" data-lang="${language.code}"${language.code === current.code ? ' aria-current="true"' : ''}>${flag(language.flag)}${escapeHtml(language.name)}</a>`,
  ).join('\n              ');
  // Two groups: the links to the help, the command line and the source, and (set off at the right) the settings of the page.
  return `<div class="nav-links">
          <details class="menu">
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
          <a href="${REPOSITORY}">${osIcon(siGithub.path)}${escapeHtml(m['nav.github'])}</a>
          </div>
          <div class="nav-tools">
          <details class="menu lang">
            <summary aria-label="${escapeHtml(m['nav.language'])}: ${escapeHtml(current.name)}">${flag(current.flag)}${escapeHtml(current.name)}</summary>
            <div class="menu-list">
              ${languages}
            </div>
          </details>
          <button type="button" class="theme-switch" disabled title="${escapeHtml(m['nav.theme'])}" aria-label="${escapeHtml(m['nav.theme'])}">${THEME_ICON}</button>
          </div>`;
}

/** A help page: the list of topics on the left, the page on the right. */
export function helpHtml(m: Messages, slug: string, prefix: string, lang: string = 'en'): string {
  const e = (key: string) => escapeHtml(m[key]);
  const links = HELP_TOPICS.map(
    (topic) =>
      `<li><a href="${prefix}${langDir(lang)}help/${topic.slug}/"${topic.slug === slug ? ' aria-current="page"' : ''}>${e(`help.${topic.key}.title`)}</a></li>`,
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
          ${helpBodyHtml(m, key, prefix + langDir(lang))}
        </article>
      </div>`;
}

/** The logo and name of the site in the top bar, leading to the start page of the language. */
export function brandHtml(m: Messages, prefix: string, lang: string): string {
  const home = prefix + langDir(lang);
  return `<a class="brand" href="${home}"><img src="${prefix}favicon.png" alt="" width="36" height="36" /><span>${escapeHtml(m['app.name'])}</span></a>`;
}

/** The note on a page that an AI translated (and a person has not read yet): empty for the other languages. */
export function noticeHtml(m: Messages, lang: string): string {
  const language = LANGUAGES.find((entry) => entry.code === lang);
  return language?.ai ? `<p class="ai-note" role="note">${inlineHtml(m['lang.ai_note'], '')}</p>` : '';
}

/** A check mark for the list of what the page can do (decoration). */
const TICK =
  '<svg class="tick" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m5 12.5 4.5 4.5L19 7"/></svg>';

/**
 * The text under the converter. It is all in the page's HTML for a search engine, but laid out so that a reader is
 * not swamped: the steps as numbered cards, what the page can do as a short list with check marks, and each
 * question folded away until it is asked (a `<details>` needs no script).
 */
export function aboutHtml(m: Messages): string {
  const e = (key: string) => escapeHtml(m[key]);
  const steps = Array.from({ length: 3 }, (_, i) => `          <li>${e(`about.how_${i + 1}`)}</li>`).join('\n');
  const can = Array.from({ length: 5 }, (_, i) => `          <li>${TICK}<span>${e(`about.can_${i + 1}`)}</span></li>`).join('\n');
  const questions = ['free', 'private', 'sizes', 'favicon', 'formats', 'cli']
    .map((id) => `<details class="qa"><summary>${e(`about.q_${id}`)}</summary>\n          <p>${e(`about.a_${id}`)}</p></details>`)
    .join('\n        ');
  return `<h2>${e('about.title')}</h2>
        <p>${e('about.intro')}</p>
        <h2>${e('about.how_title')}</h2>
        <ol>
${steps}
        </ol>
        <h2>${e('about.can_title')}</h2>
        <ul class="can">
${can}
        </ul>
        <h2>${e('about.faq_title')}</h2>
        <div class="qas">
        ${questions}
        </div>
        <p class="source"><a href="${REPOSITORY}">${e('about.source')}</a></p>`;
}

export function robotsTxt(siteUrl: string): string {
  return `User-agent: *\nAllow: /\n${siteUrl ? `\nSitemap: ${siteUrl}sitemap.xml\n` : ''}`;
}

/** Every page in every language, each with the same page in the other languages (for search engines). */
export function sitemapXml(siteUrl: string): string {
  const pages = ['', ...HELP_TOPICS.map((topic) => `help/${topic.slug}/`)];
  const at = (code: string, page: string) => escapeHtml(siteUrl + langDir(code) + page);
  const urls = LANGUAGES.flatMap((language) =>
    pages.map((page) => {
      const alternates = [
        ...LANGUAGES.map((other) => `<xhtml:link rel="alternate" hreflang="${langTag(other.code)}" href="${at(other.code, page)}"/>`),
        `<xhtml:link rel="alternate" hreflang="x-default" href="${at('en', page)}"/>`,
      ].join('');
      return `  <url><loc>${at(language.code, page)}</loc>${alternates}</url>`;
    }),
  ).join('\n');
  return `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">\n${urls}\n</urlset>\n`;
}

/** Which language and which help page an address of the site is ("/de/help/privacy/index.html"). */
export function locate(path: string): { lang: LanguageCode; slug?: string; depth: number } {
  const codes = LANGUAGES.filter((language) => language.code !== 'en')
    .map((language) => language.code)
    .join('|');
  const match = new RegExp(`^/(?:(${codes})/)?(?:help/([^/]+)/)?`).exec(path);
  const lang = (match?.[1] ?? 'en') as LanguageCode;
  const slug = match?.[2];
  return { lang, slug, depth: (match?.[1] ? 1 : 0) + (slug ? 2 : 0) };
}

/** Fills the placeholders of index.html and writes robots.txt (and sitemap.xml when the address is known). */
export function seo(address?: string): Plugin {
  const siteUrl = normalizeSiteUrl(address);
  return {
    name: 'img2ico-seo',
    transformIndexHtml(html, context) {
      // A help page is two folders deep, a language one more; the converter is at the root of its language.
      const { lang, slug, depth } = locate(context.path);
      const m = CATALOGUES[lang];
      const page = slug ? helpPageInfo(m, slug) : undefined;
      const prefix = depth ? '../'.repeat(depth) : './';
      return html
        .replace('<html lang="en">', () => `<html lang="${langTag(lang)}">`)
        .replace('<!--seo:head-->', () => headTags(m, siteUrl, page, lang, prefix))
        .replace('<!--seo:brand-->', () => brandHtml(m, prefix, lang))
        .replace('<!--seo:notice-->', () => noticeHtml(m, lang))
        .replace('<!--seo:help-->', () => (slug ? helpHtml(m, slug, prefix, lang) : ''))
        .replace('<!--seo:header-->', () => headerHtml(m))
        .replace('<!--seo:art-->', () => artHtml())
        .replace('<!--seo:nav-->', () => navHtml(m, prefix, lang, page ? page.path : ''))
        .replace('<!--seo:noscript-->', () => escapeHtml(m['seo.noscript']))
        .replace('<!--seo:privacy-->', () => escapeHtml(m['app.privacy']))
        .replace('<!--seo:about-->', () => aboutHtml(m));
    },
    generateBundle() {
      this.emitFile({ type: 'asset', fileName: 'robots.txt', source: robotsTxt(siteUrl) });
      if (siteUrl) this.emitFile({ type: 'asset', fileName: 'sitemap.xml', source: sitemapXml(siteUrl) });
    },
  };
}
