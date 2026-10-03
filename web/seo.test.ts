import { describe, expect, it } from 'vitest';
import { CATALOGUES } from './catalogues';
import { en } from './src/i18n/en';
import {
  HELP_TOPICS,
  LANGUAGES,
  aboutHtml,
  headTags as headTagsFor,
  helpHtml,
  helpPageInfo,
  langDir,
  languageRedirect,
  locate,
  navHtml,
  noticeHtml,
  escapeHtml,
  headTags,
  headerHtml,
  jsonForScript,
  normalizeSiteUrl,
  robotsTxt,
  sitemapXml,
  structuredData,
} from './seo';

describe('the address of the site', () => {
  it('ends in one slash, or is left out', () => {
    expect(normalizeSiteUrl('https://example.org/img2ico')).toBe('https://example.org/img2ico/');
    expect(normalizeSiteUrl('https://example.org///')).toBe('https://example.org/');
    expect(normalizeSiteUrl('')).toBe('');
    expect(normalizeSiteUrl(undefined)).toBe('');
    expect(normalizeSiteUrl('javascript:alert(1)')).toBe('');
    expect(normalizeSiteUrl('example.org')).toBe('');
  });
});

describe('the head tags', () => {
  it('have a title and description of a sensible length', () => {
    expect(en['seo.title'].length).toBeLessThanOrEqual(65);
    expect(en['seo.description'].length).toBeLessThanOrEqual(160);
    const tags = headTags(en, '');
    expect(tags).toContain(`<title>${en['seo.title']}</title>`);
    expect(tags).toContain('name="description"');
    expect(tags).toContain('property="og:title"');
  });

  it('name the canonical address and the preview picture only when it is known', () => {
    const without = headTags(en, '');
    expect(without).not.toContain('canonical');
    expect(without).not.toContain('og:image"');
    const withUrl = headTags(en, 'https://example.org/img2ico/');
    expect(withUrl).toContain('<link rel="canonical" href="https://example.org/img2ico/" />');
    expect(withUrl).toContain('content="https://example.org/img2ico/og-image.png"');
  });

  it('carry structured data that cannot break out of its element', () => {
    const data = structuredData(en, 'https://example.org/') as Record<string, unknown>;
    expect(data['@type']).toBe('WebApplication');
    expect(data.isAccessibleForFree).toBe(true);
    expect(jsonForScript({ a: '</script><b>' })).not.toContain('</script>');
    expect(JSON.parse(jsonForScript({ a: '</script>' })).a).toBe('</script>');
  });
});

describe('the text of the page', () => {
  it('has one heading level 1 and escapes what it prints', () => {
    expect(headerHtml(en).match(/<h1>/g)).toHaveLength(1);
    expect(escapeHtml('a<b & "c"')).toBe('a&lt;b &amp; &quot;c&quot;');
    expect(headerHtml({ ...en, 'hero.title': '<x>' })).toContain('&lt;x&gt;');
  });

  it('has every question with its answer', () => {
    const html = aboutHtml(en);
    for (const id of ['free', 'private', 'sizes', 'favicon', 'formats', 'cli']) {
      expect(html).toContain(escapeHtml(en[`about.q_${id}` as keyof typeof en]));
      expect(html).toContain(escapeHtml(en[`about.a_${id}` as keyof typeof en]));
    }
    expect(html).not.toContain('about.');
  });
});

describe('robots.txt and sitemap.xml', () => {
  it('allow everything, and point to the sitemap when there is one', () => {
    expect(robotsTxt('')).toBe('User-agent: *\nAllow: /\n');
    expect(robotsTxt('https://example.org/')).toContain('Sitemap: https://example.org/sitemap.xml');
    expect(sitemapXml('https://example.org/')).toContain('<loc>https://example.org/</loc>');
  });
});

describe('the help pages and the top bar menus', () => {
  it('lists every help topic in the menu and on each page, with a working address from any depth', () => {
    for (const prefix of ['./', '../../']) {
      const nav = navHtml(en, prefix);
      for (const topic of HELP_TOPICS) {
        expect(nav).toContain(`href="${prefix}help/${topic.slug}/"`);
        expect(nav).toContain(en[`help.${topic.key}.title`]);
      }
    }
    const page = helpHtml(CATALOGUES.en, 'privacy', '../../');
    for (const topic of HELP_TOPICS) expect(page).toContain(`href="../../help/${topic.slug}/"`);
    expect(page).toContain('aria-current="page"');
    expect(page).toContain(`<h1>${en['help.privacy.title']}</h1>`);
  });

  it('shows the five languages as links to the same page, and a switch for the theme that the script wakes up', () => {
    const nav = navHtml(en, '../../', 'en', 'help/privacy/');
    expect(LANGUAGES.map((language) => language.code)).toEqual(['en', 'de', 'es', 'pt-br', 'fr']);
    for (const language of LANGUAGES) expect(nav).toContain(language.name);
    expect(nav).toContain('href="../../help/privacy/" lang="en"');
    expect(nav).toContain('href="../../de/help/privacy/" lang="de" hreflang="de" data-lang="de"');
    expect(nav).toContain('href="../../pt-br/help/privacy/"');
    expect(nav.match(/aria-current="true"/g)).toHaveLength(1);
    expect(nav).not.toContain('aria-disabled');
    expect(nav).toContain('class="theme-switch" disabled');
  });

  it('keeps the other languages in their own folder: the help menu, the current language and the way back to the root', () => {
    const nav = navHtml(en, '../../../', 'de', 'help/privacy/');
    expect(nav).toContain('href="../../../de/help/privacy/"');
    expect(nav).toContain('href="../../../help/privacy/" lang="en"');
    expect(nav).toMatch(/<a href="..\/..\/..\/de\/help\/privacy\/" lang="de"[^>]*aria-current="true"/);
    expect(nav).toContain('aria-label="Language: Deutsch"');
    expect(helpHtml(CATALOGUES.en, 'privacy', '../../../', 'de')).toContain('href="../../../de/help/settings/"');
  });

  it('gives each help page its own title, description and address', () => {
    const info = helpPageInfo(en, 'file-types');
    expect(info?.path).toBe('help/file-types/');
    expect(info?.title).toContain(en['help.types.title']);
    expect(helpPageInfo(en, 'nonsense')).toBeUndefined();
    const tags = headTagsFor(en, 'https://example.org/img2ico/', info);
    expect(tags).toContain('href="https://example.org/img2ico/help/file-types/"');
    expect(tags).not.toContain('application/ld+json');
  });

  it('puts every help page in the sitemap', () => {
    const map = sitemapXml('https://example.org/img2ico/');
    for (const topic of HELP_TOPICS) expect(map).toContain(`https://example.org/img2ico/help/${topic.slug}/`);
  });
});

describe('the languages', () => {
  it('have a folder each, English at the root', () => {
    expect(langDir('en')).toBe('');
    expect(langDir('pt-br')).toBe('pt-br/');
  });

  it('are told apart by the address of a page', () => {
    expect(locate('/')).toEqual({ lang: 'en', slug: undefined, depth: 0 });
    expect(locate('/index.html')).toEqual({ lang: 'en', slug: undefined, depth: 0 });
    expect(locate('/help/privacy/index.html')).toEqual({ lang: 'en', slug: 'privacy', depth: 2 });
    expect(locate('/de/')).toEqual({ lang: 'de', slug: undefined, depth: 1 });
    expect(locate('/pt-br/help/settings/index.html')).toEqual({ lang: 'pt-br', slug: 'settings', depth: 3 });
  });

  it('tell search engines where the same page is in the other languages, and its language', () => {
    const tags = headTags(CATALOGUES.de, 'https://example.org/img2ico/', helpPageInfo(CATALOGUES.de, 'privacy'), 'de', '../../../');
    expect(tags).toContain('<link rel="canonical" href="https://example.org/img2ico/de/help/privacy/" />');
    for (const language of LANGUAGES) {
      expect(tags).toContain(`hreflang="${language.code}" href="https://example.org/img2ico/${langDir(language.code)}help/privacy/"`);
    }
    expect(tags).toContain('hreflang="x-default" href="https://example.org/img2ico/help/privacy/"');
    expect(tags).toContain('<meta property="og:locale" content="de_DE" />');
    expect(tags).toContain('href="../../../favicon.ico"');
    expect(headTags(CATALOGUES['pt-br'], '', undefined, 'pt-br')).toContain('content="pt_BR"');
  });

  it('send visitors of an English page to theirs, and no other page', () => {
    expect(headTags(en, '', undefined, 'en', './')).toContain("location.replace('./'+l+'/')");
    expect(headTags(en, '', helpPageInfo(en, 'privacy'), 'en', '../../')).toContain("location.replace('../../'+l+'/help/privacy/')");
    expect(headTags(CATALOGUES.de, '', undefined, 'de', '../')).not.toContain('location.replace');
    const script = languageRedirect('./', '');
    expect(script).toContain('"pt":"pt-br"');
    expect(script).toContain("localStorage.getItem('img2ico.lang.v1')");
  });

  it('say on the pages an AI translated that it did, and on no other', () => {
    expect(noticeHtml(CATALOGUES.es, 'es')).toContain('class="ai-note"');
    expect(noticeHtml(CATALOGUES.es, 'es')).toContain('href="https://github.com/Matek85/img2ico/issues"');
    expect(noticeHtml(en, 'en')).toBe('');
    expect(noticeHtml(CATALOGUES.de, 'de')).toBe('');
  });

  it('are all in the sitemap, each page with the same page in the other languages', () => {
    const map = sitemapXml('https://example.org/img2ico/');
    for (const language of LANGUAGES) {
      expect(map).toContain(`<loc>https://example.org/img2ico/${langDir(language.code)}</loc>`);
      for (const topic of HELP_TOPICS) expect(map).toContain(`<loc>https://example.org/img2ico/${langDir(language.code)}help/${topic.slug}/</loc>`);
    }
    expect(map).toContain('xmlns:xhtml');
    expect(map).toContain('<xhtml:link rel="alternate" hreflang="fr" href="https://example.org/img2ico/fr/"/>');
  });
});

describe('the theme the visitor chose', () => {
  it('is set before the page is drawn, on every page', () => {
    for (const tags of [headTags(en, ''), headTags(en, '', helpPageInfo(en, 'privacy'))]) {
      expect(tags).toContain("localStorage.getItem('img2ico.theme.v1')");
      expect(tags).toContain('document.documentElement.dataset.theme');
    }
  });
});
