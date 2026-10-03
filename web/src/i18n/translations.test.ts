import { afterEach, describe, expect, it } from 'vitest';
import { de } from './de';
import { en } from './en';
import { es } from './es';
import { fr } from './fr';
import { helpDe } from './help-de';
import { helpEn } from './help-en';
import { helpEs } from './help-es';
import { helpFr } from './help-fr';
import { helpPtBr } from './help-pt-br';
import { loadLocale, locale, t } from './index';
import { ptBr } from './pt-br';

type Table = Record<string, string>;

const LANGUAGES: { code: string; app: Table; help: Table }[] = [
  { code: 'de', app: de, help: helpDe },
  { code: 'es', app: es, help: helpEs },
  { code: 'pt-br', app: ptBr, help: helpPtBr },
  { code: 'fr', app: fr, help: helpFr },
];

const placeholders = (text: string) => [...text.matchAll(/\{(\w+)\}/g)].map((match) => match[1]).sort();
const marks = (text: string) => ({
  bold: (text.match(/\*\*[^*]+\*\*/g) ?? []).length,
  code: [...text.matchAll(/`([^`]+)`/g)].map((match) => match[1]).sort(),
  keys: (text.match(/\[\[[^\]]+\]\]/g) ?? []).length,
  help: [...text.matchAll(/\]\(help:([a-z-]+)\)/g)].map((match) => match[1]).sort(),
  links: [...text.matchAll(/\]\((https:\/\/[^)\s]+)\)/g)].map((match) => match[1]).sort(),
});

// Texts that read the same in every language: names, numbers and symbols.
const SAME_EVERYWHERE = new Set([
  'app.name', 'nav.github', 'nav.cli_windows', 'nav.cli_macos', 'nav.cli_linux', 'turn.degrees', 'keys.numpad', 'keys.num_range',
  'msg.other', 'pixels.hex', 'result.file', 'pixels.zoom', 'queue.sub_detail_none', 'editor.source',
]);

// Help texts that are names of file types and systems, the same in every language.
const NAMES_ONLY = new Set(['help.types.02.dl.4.t', 'help.types.05.dl.1.t', 'help.types.05.dl.2.t']);

describe.each(LANGUAGES)('the translation $code', ({ code, app, help }) => {
  it('has exactly the keys of English: none missing, none extra', () => {
    expect(Object.keys(app).sort()).toEqual(Object.keys(en).sort());
    expect(Object.keys(help).sort()).toEqual(Object.keys(helpEn).sort());
  });

  it('has no empty text', () => {
    for (const [key, text] of [...Object.entries(app), ...Object.entries(help)]) expect(text.trim().length, key).toBeGreaterThan(0);
  });

  it('uses the same {values} as English in every text', () => {
    for (const [key, text] of Object.entries(app)) expect(placeholders(text), `${code} ${key}`).toEqual(placeholders(en[key as keyof typeof en]));
  });

  it('keeps the marks, the keys, the code and the links of the help texts', () => {
    for (const [key, text] of Object.entries(help)) {
      expect(marks(text), `${code} ${key}`).toEqual(marks(helpEn[key as keyof typeof helpEn]));
    }
  });

  it('has the plural forms English has', () => {
    const pluralOf = (table: Table) => Object.keys(table).filter((key) => /_(one|other)$/.test(key)).sort();
    expect(pluralOf(app)).toEqual(pluralOf(en));
  });

  it('is really translated: it differs from English except for names and symbols', () => {
    const same = Object.entries(app)
      .filter(([key, text]) => text === en[key as keyof typeof en] && !SAME_EVERYWHERE.has(key))
      .map(([key]) => key);
    // A word that is the same in two languages (Standard, Pixels, Original, ...) is allowed, but not a sentence.
    // (The words are counted without the {values} and symbols: "{width} × {height} pixels" is one word.)
    const words = (text: string) => text.replace(/\{\w+\}/g, ' ').split(/[^\p{L}]+/u).filter(Boolean);
    for (const key of same) expect(words(en[key as keyof typeof en]).length, `${code} ${key} is still English`).toBeLessThanOrEqual(2);
    expect(same.length).toBeLessThan(25);
    const sameHelp = Object.entries(help).filter(([key, text]) => text === helpEn[key as keyof typeof helpEn] && text.length > 12 && !NAMES_ONLY.has(key));
    expect(sameHelp.map(([key]) => key)).toEqual([]);
  });

  it('points to the issues of the project in the note about the AI translation', () => {
    expect(app['lang.ai_note']).toContain('](https://github.com/Matek85/img2ico/issues)');
  });
});

describe('the French typography', () => {
  const texts = [...Object.entries(fr), ...Object.entries(helpFr)].map(([key, text]) => [key, text.replace(/https?:\/\/\S+/g, '').replace(/`[^`]*`/g, '')] as const);

  it('keeps a no-break space before : ; ? ! and %, and inside « »', () => {
    for (const [key, text] of texts) {
      expect(text, key).not.toMatch(/ [:;?!%]/);
      expect(text, key).not.toMatch(/« | »/);
    }
  });

  it('uses the typographic apostrophe', () => {
    for (const [key, text] of texts) expect(text, key).not.toMatch(/[a-zé]'[a-zéàè]/i);
  });
});

describe('loading a language in the browser', () => {
  afterEach(async () => {
    await loadLocale('en');
  });

  it('switches the texts and the plural rules to that language', async () => {
    await loadLocale('de');
    expect(locale()).toBe('de');
    expect(t('nav.help')).toBe('Hilfe');
    expect(t('queue.title', { count: 1 })).toBe('Deine Warteschlange: 1 Icon');
    expect(t('queue.title', { count: 3 })).toBe('Deine Warteschlange: 3 Icons');
    await loadLocale('pt-br');
    expect(locale()).toBe('pt-br');
    expect(t('nav.help')).toBe('Ajuda');
    await loadLocale('fr');
    expect(t('queue.title', { count: 0 })).toBe('Votre file d’attente : 0 icône');
    await loadLocale('es');
    expect(t('queue.title', { count: 2 })).toBe('Tu cola: 2 iconos');
  });

  it('falls back to English for a language the page does not have', async () => {
    await loadLocale('de');
    await loadLocale('ja');
    expect(locale()).toBe('en');
    expect(t('nav.help')).toBe('Help');
  });
});
