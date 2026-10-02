import { describe, expect, it } from 'vitest';
import { en } from './src/i18n/en';
import {
  aboutHtml,
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
    expect(headerHtml({ ...en, 'app.name': '<x>' })).toContain('&lt;x&gt;');
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
