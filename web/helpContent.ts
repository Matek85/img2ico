// The body of a help page: built from the message catalogue at build time, so it is plain HTML
// that search engines and people without scripts read. A page is the keys
//   help.<page>.<nn>.<kind>[.<item>[.t|d]]
// in the order of the numbers (see src/i18n/help-en.ts for what they look like).
import { CROP_KEYS, GROUPS, SHORTCUTS } from './src/lib/shortcuts.ts';

type Messages = Record<string, string>;

export function escapeHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

export type Block =
  | { kind: 'h2' | 'h3' | 'p' | 'note' | 'keys'; text: string }
  | { kind: 'ul' | 'ol'; items: string[] }
  | { kind: 'dl'; items: { term: string; text: string }[] };

const KEY = /^help\.([a-z]+)\.(\d\d)\.(h2|h3|p|note|keys|ul|ol|dl)(?:\.(\d+))?(?:\.(t|d))?$/;

/** The keys of the catalogue that look like a block of a help page (a misspelled one is found by the tests). */
export function isHelpBlockKey(key: string): boolean {
  return KEY.test(key);
}

/** The blocks of the page `page` ("start", "settings", ...), in order. */
export function helpBlocks(m: Messages, page: string): Block[] {
  const numbered = new Map<string, { kind: string; entries: { item: number; part: string; text: string }[] }>();
  for (const [key, text] of Object.entries(m)) {
    const match = KEY.exec(key);
    if (!match || match[1] !== page) continue;
    const [, , number, kind, item, part] = match;
    const block = numbered.get(number) ?? { kind, entries: [] };
    block.entries.push({ item: Number(item ?? 0), part: part ?? '', text });
    numbered.set(number, block);
  }
  return [...numbered.entries()]
    .sort(([a], [b]) => Number(a) - Number(b))
    .map(([, { kind, entries }]): Block => {
      entries.sort((a, b) => a.item - b.item);
      if (kind === 'ul' || kind === 'ol') return { kind, items: entries.map((entry) => entry.text) };
      if (kind === 'dl') {
        const items = new Map<number, { term: string; text: string }>();
        for (const entry of entries) {
          const row = items.get(entry.item) ?? { term: '', text: '' };
          if (entry.part === 't') row.term = entry.text;
          else row.text = entry.text;
          items.set(entry.item, row);
        }
        return { kind: 'dl', items: [...items.values()] };
      }
      return { kind: kind as 'h2' | 'h3' | 'p' | 'note' | 'keys', text: entries[0].text };
    });
}

/**
 * A text as HTML. The text is escaped first; then **bold**, `code`, [[Key]], [words](help:page) and
 * [words](https://...) are marked up. `prefix` leads from the page to the root of the site.
 */
export function inlineHtml(text: string, prefix: string): string {
  return escapeHtml(text)
    .replace(/`([^`]+)`/g, '<code>$1</code>')
    .replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
    .replace(/\[\[([^\]]+)\]\]/g, '<kbd>$1</kbd>')
    .replace(/\[([^\]]+)\]\(help:([a-z-]+)\)/g, (_whole, label: string, slug: string) => `<a href="${prefix}help/${slug}/">${label}</a>`)
    .replace(/\[([^\]]+)\]\((https:\/\/[^)\s]+)\)/g, '<a href="$2" rel="noopener">$1</a>');
}

/** Every shortcut of the editor, by group, as it is in the list the page shows with "?". */
function keysHtml(m: Messages, label: string): string {
  const entry = (keyLabel: string, text: string) => `<div><dt><kbd>${escapeHtml(keyLabel)}</kbd></dt><dd>${escapeHtml(text)}</dd></div>`;
  const group = (title: string, rows: string[]) => `<section><h2>${escapeHtml(title)}</h2><dl class="key-list">${rows.join('')}</dl></section>`;
  const groups = GROUPS.map((name) =>
    group(
      m[`keys.group_${name}`],
      SHORTCUTS.filter((shortcut) => shortcut.group === name).map((shortcut) => entry(shortcut.label, m[`keys.act_${shortcut.id}`])),
    ),
  );
  groups.push(group(m['keys.group_crop'], CROP_KEYS.map((key) => entry(key.label, m[`keys.crop_${key.action}`]))));
  return `<div class="key-groups" role="group" aria-label="${escapeHtml(label)}">\n          ${groups.join('\n          ')}\n        </div>`;
}

function blockHtml(block: Block, m: Messages, prefix: string): string {
  const inline = (text: string) => inlineHtml(text, prefix);
  switch (block.kind) {
    case 'h2':
    case 'h3':
      return `<${block.kind}>${inline(block.text)}</${block.kind}>`;
    case 'p':
      return `<p>${inline(block.text)}</p>`;
    case 'note':
      return `<p class="note">${inline(block.text)}</p>`;
    case 'ul':
    case 'ol':
      return `<${block.kind}>${block.items.map((item) => `<li>${inline(item)}</li>`).join('')}</${block.kind}>`;
    case 'dl':
      return `<dl class="terms">${block.items.map((row) => `<div><dt>${inline(row.term)}</dt><dd>${inline(row.text)}</dd></div>`).join('')}</dl>`;
    case 'keys':
      return keysHtml(m, block.text);
  }
}

/** The blocks of a help page as HTML. */
export function helpBodyHtml(m: Messages, page: string, prefix: string): string {
  return helpBlocks(m, page)
    .map((block) => blockHtml(block, m, prefix))
    .join('\n          ');
}
