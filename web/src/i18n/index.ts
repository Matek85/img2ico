import { en, type MessageKey } from './en';

// The language in use. Each language page of the site (/, /de/, ...) is built in its own language and says so in
// `<html lang>`; the page loads the texts of that language (English is always there, for the tests and as the start).
// A new language is a new file with the same keys as en.ts, listed here and in seo.ts.
type Messages = Record<MessageKey, string>;
const loaders: Record<string, () => Promise<Messages>> = {
  de: () => import('./de').then((module) => module.de),
  es: () => import('./es').then((module) => module.es),
  'pt-br': () => import('./pt-br').then((module) => module.ptBr),
  fr: () => import('./fr').then((module) => module.fr),
};

let tag = 'en';
let messages: Messages = en;

/** Loads the texts of a language ("de", "pt-br"); a language the page does not have is English. */
export async function loadLocale(code: string): Promise<void> {
  const wanted = code.toLowerCase();
  const load = loaders[wanted];
  if (!load) {
    tag = 'en';
    messages = en;
    return;
  }
  messages = await load();
  tag = wanted;
}

/** The language in use, as a language tag ("en"). */
export function locale(): string {
  return tag;
}

export type Params = Record<string, string | number>;

/**
 * The text for `key` with `{name}` placeholders filled in. When the
 * parameters contain a `count`, the plural form of the key (`key_one`,
 * `key_other`, ...) for the current language is used if there is one.
 */
export function t(key: MessageKey | string, params: Params = {}): string {
  const table = messages as Record<string, string>;
  let template = table[key];
  if (typeof params.count === 'number') {
    const rule = new Intl.PluralRules(tag).select(params.count);
    template = table[`${key}_${rule}`] ?? table[`${key}_other`] ?? template;
  }
  if (template === undefined) {
    return key;
  }
  return template.replace(/\{(\w+)\}/g, (whole, name: string) =>
    name in params ? String(params[name]) : whole,
  );
}

/** A byte count as people read it: "1.4 KB". */
export function formatBytes(bytes: number): string {
  const units = ['B', 'KB', 'MB', 'GB'];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  const digits = unit === 0 || value >= 100 ? 0 : 1;
  return `${new Intl.NumberFormat(tag, { maximumFractionDigits: digits }).format(value)} ${units[unit]}`;
}
