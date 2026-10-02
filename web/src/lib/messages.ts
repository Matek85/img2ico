// The engine is shared with the command line, so its warnings and errors are sentences
// that name command line options ("--tolerance") and are made for a terminal. This turns
// them into sentences for this page. Only what is shown changes, never the engine.
// A sentence it does not know is shown as it is, with the option names put into words.

import { t } from '../i18n';

type Rule = [pattern: RegExp, key: string, params: (match: RegExpMatchArray) => Record<string, string | number>];

const RULES: Rule[] = [
  [/nothing was removed - no pixel connected to the image border is close to (\S+?)\./, 'msg.bg_nothing', (m) => ({ color: m[1] })],
  [/only ([\d.]+)% of the image matched the background color (\S+) - almost nothing/, 'msg.bg_little', (m) => ({ percent: m[1], color: m[2] })],
  [/([\d.]+)% of the image matched the background color (\S+) - almost everything/, 'msg.bg_much', (m) => ({ percent: m[1], color: m[2] })],
  [
    /Could not detect a single background color: the most common color along the border, (\S+), covers only (\d+)% of it \(within a tolerance of (\d+)%\)/,
    'msg.bg_detect',
    (m) => ({ color: m[1], percent: m[2], tolerance: m[3] }),
  ],
  [/--trim found no transparent margin/, 'msg.trim_none', () => ({})],
  [/--trim found nothing to keep/, 'msg.trim_empty', () => ({})],
  [/at these sizes, only a thin sliver of the actual artwork will be visible: (.*?) - this can be caused/, 'msg.sliver', (m) => ({ sizes: m[1] })],
  [
    /seed point \((\d+),(\d+)\) is outside the image \((\d+)x(\d+)\)/,
    'msg.seed',
    (m) => ({ x: m[1], y: m[2], width: m[3], height: m[4] }),
  ],
  [
    /the image is (\d+)x(\d+) pixels \(([\d.]+) megapixels\), more than the limit of ([\d.]+) megapixels/,
    'msg.too_big',
    (m) => ({ width: m[1], height: m[2], megapixels: m[3], limit: m[4] }),
  ],
];

// Option names, in words, for a sentence no rule knows.
const OPTIONS: Record<string, string> = {
  tolerance: 'msg.option_tolerance',
  'chroma-key': 'msg.option_background',
  find: 'msg.option_background',
  trim: 'msg.option_trim',
  padding: 'msg.option_padding',
  crop: 'msg.option_crop',
  seed: 'msg.option_point',
};

/** A message of the engine, as a sentence for this page. */
export function friendly(message: string): string {
  const text = message.replace(/^Warning:\s*/, '').trim();
  for (const [pattern, key, params] of RULES) {
    const match = text.match(pattern);
    if (match) return t(key, params(match));
  }
  const words = text.replace(/--([a-z][a-z-]*)/g, (_, name: string) => (OPTIONS[name] ? t(OPTIONS[name]) : name.replace(/-/g, ' ')));
  return words.charAt(0).toUpperCase() + words.slice(1);
}

/** The text of whatever was thrown, as a sentence for this page. */
export function explain(error: unknown): string {
  return friendly(error instanceof Error ? error.message : String(error));
}
