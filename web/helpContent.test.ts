import { describe, expect, it } from 'vitest';
import { CATALOGUES } from './catalogues';
import { CROP_KEYS, GROUPS, SHORTCUTS } from './src/lib/shortcuts';
import { type Block, helpBlocks, helpBodyHtml, inlineHtml, isHelpBlockKey } from './helpContent';
import { HELP_TOPICS } from './seo';

const messages = CATALOGUES.en;

const textsOf = (block: Block): string[] =>
  'text' in block ? [block.text] : block.kind === 'dl' ? block.items.flatMap((row) => [row.term, row.text]) : block.items;

describe('the marks in a text', () => {
  it('turns bold, code, keys and links into HTML', () => {
    expect(inlineHtml('Press **Done** or [[Esc]]; `--what-if` shows it.', './')).toBe(
      'Press <strong>Done</strong> or <kbd>Esc</kbd>; <code>--what-if</code> shows it.',
    );
    expect(inlineHtml('See [privacy](help:privacy).', '../../')).toBe('See <a href="../../help/privacy/">privacy</a>.');
    expect(inlineHtml('[GitHub](https://github.com/Matek85/img2ico)', './')).toBe(
      '<a href="https://github.com/Matek85/img2ico" rel="noopener">GitHub</a>',
    );
  });

  it('escapes everything else, so a text can never add markup of its own', () => {
    expect(inlineHtml('<script>alert("x")</script> & more', './')).toBe(
      '&lt;script&gt;alert(&quot;x&quot;)&lt;/script&gt; &amp; more',
    );
    expect(inlineHtml('[x](javascript:alert(1))', './')).not.toContain('<a');
    expect(inlineHtml('[x](http://insecure.example)', './')).not.toContain('<a');
  });
});

describe('the blocks of a page', () => {
  it('are in the order of their numbers, with a list gathered from its items and a term with its text', () => {
    const m = {
      'help.demo.02.ul.2': 'second',
      'help.demo.02.ul.1': 'first',
      'help.demo.01.h2': 'Title',
      'help.demo.03.dl.1.d': 'what A is',
      'help.demo.03.dl.1.t': 'A',
      'help.other.01.p': 'elsewhere',
    };
    expect(helpBlocks(m, 'demo')).toEqual([
      { kind: 'h2', text: 'Title' },
      { kind: 'ul', items: ['first', 'second'] },
      { kind: 'dl', items: [{ term: 'A', text: 'what A is' }] },
    ]);
  });
});

describe('the help texts of the catalogue', () => {
  const helpKeys = Object.keys(messages).filter((key) => /^help\.[a-z]+\.\d/.test(key));

  it('all have the shape of a block (a misspelled key would leave its text out of the page)', () => {
    expect(helpKeys.length).toBeGreaterThan(50);
    for (const key of helpKeys) expect(isHelpBlockKey(key), key).toBe(true);
  });

  it('give every topic a page that starts with text and has no empty text', () => {
    for (const topic of HELP_TOPICS) {
      const blocks = helpBlocks(messages, topic.key);
      expect(blocks.length, topic.slug).toBeGreaterThan(2);
      expect(['p', 'h2', 'keys']).toContain(blocks[0].kind);
      for (const block of blocks) {
        const texts = textsOf(block);
        for (const text of texts) expect(text.trim().length, `${topic.slug}: ${JSON.stringify(block).slice(0, 60)}`).toBeGreaterThan(0);
      }
    }
  });

  it('link only to help pages that exist', () => {
    const slugs: string[] = HELP_TOPICS.map((topic) => topic.slug);
    for (const key of helpKeys) {
      for (const [, slug] of messages[key].matchAll(/\]\(help:([a-z-]+)\)/g)) expect(slugs, `${key} -> ${slug}`).toContain(slug);
    }
  });

  it('are used by a page: every number of a topic is there', () => {
    const pages: string[] = HELP_TOPICS.map((topic) => topic.key);
    for (const key of helpKeys) expect(pages, key).toContain(key.split('.')[1]);
  });

  it('make a page with no placeholder left', () => {
    for (const topic of HELP_TOPICS) {
      const html = helpBodyHtml(messages, topic.key, '../../');
      expect(html).not.toContain('still being written');
      expect(html.length).toBeGreaterThan(500);
    }
  });
});

describe('the page of the keyboard shortcuts', () => {
  const html = helpBodyHtml(messages, 'shortcuts', '../../');

  it('lists every shortcut with its text', () => {
    for (const group of GROUPS) expect(html).toContain(`<h2>${messages[`keys.group_${group}`]}</h2>`);
    for (const shortcut of SHORTCUTS) {
      expect(html, shortcut.id).toContain(`<kbd>${shortcut.label}</kbd></dt><dd>${messages[`keys.act_${shortcut.id}`]}</dd>`);
    }
    for (const key of CROP_KEYS) expect(html, key.action).toContain(messages[`keys.crop_${key.action}`]);
  });
});
