import { describe, expect, it } from 'vitest';
import { en } from './src/i18n/en';
import {
  HELP_TOPICS,
  LANGUAGES,
  aboutHtml,
  headTags as headTagsFor,
  helpHtml,
  helpPageInfo,
  navHtml,
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
    const page = helpHtml(en, 'privacy', '../../');
    for (const topic of HELP_TOPICS) expect(page).toContain(`href="../../help/${topic.slug}/"`);
    expect(page).toContain('aria-current="page"');
    expect(page).toContain(`<h1>${en['help.privacy.title']}</h1>`);
  });

  it('shows the five languages, with only English a link for now, and a switch for the theme that does nothing yet', () => {
    const nav = navHtml(en);
    expect(LANGUAGES.map((language) => language.code)).toEqual(['en', 'de', 'es', 'pt-br', 'fr']);
    for (const language of LANGUAGES) expect(nav).toContain(language.name);
    expect(nav.match(/aria-disabled="true"/g)).toHaveLength(4);
    expect(nav).toContain('class="theme-switch" disabled');
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

describe('the theme the visitor chose', () => {
  it('is set before the page is drawn, on every page', () => {
    for (const tags of [headTags(en, ''), headTags(en, '', helpPageInfo(en, 'privacy'))]) {
      expect(tags).toContain("localStorage.getItem('img2ico.theme.v1')");
      expect(tags).toContain('document.documentElement.dataset.theme');
    }
  });
});
