// The settings as a file: the same TOML file the command line reads with `--config` (and writes with
// `--out-toml`), so a file made on the page works on the command line and the other way round.
//
// A settings file has flat `key = value` lines (the keys are the long names of the command-line flags). The page
// writes the ones it has - also the picture's own mirror, turn, crop and GIF frame, so a file saved here gives the
// same icon here again and on the command line - and reads any file the command line reads. What the command line
// knows but the page has no use for (folders, threads, ...) is skipped and reported; a name that neither knows is
// an error, so a typo does not pass quietly (the command line only warns about it).

import { DEFAULT_SIZES, type Fit, type Format, SIZE_CHOICES, type Settings } from './settings';

/** Every key the command line reads (`KNOWN_SETTINGS_KEYS` in src/config.rs; a test keeps this list in step). */
export const CLI_KEYS = [
  'sizes', 'preset', 'chroma-key', 'tolerance', 'feather', 'seeds', 'find', 'find-min-size', 'auto-apply',
  'replace-color', 'grayscale', 'padding', 'fit', 'flip-horizontal', 'flip-vertical', 'rotate', 'crop', 'trim',
  'corner-radius', 'max-pixels', 'jobs',
  'gif-frame', 'output-format', 'delete-source', 'force', 'keep-going', 'skip-existing', 'recursive', 'include',
  'exclude', 'combine', 'index', 'silent',
] as const;

/** The keys the page uses. */
export const USED_KEYS = [
  'sizes', 'preset', 'chroma-key', 'tolerance', 'feather', 'grayscale', 'padding', 'fit', 'flip-horizontal',
  'flip-vertical', 'rotate', 'crop', 'trim', 'corner-radius', 'gif-frame', 'output-format',
] as const;

/** The most bytes a settings file may have (the largest real one is a few hundred). */
export const MAX_FILE_BYTES = 100_000;

/** The sizes of the command line's presets (`SizePreset::sizes` in src/cli.rs). */
export const PRESET_SIZES: Record<string, readonly number[]> = {
  windows: [16, 20, 24, 32, 40, 48, 64, 96, 128, 256],
  favicon: [16, 32, 48],
  minimal: [16, 32],
};

type ProblemCode =
  /** The text is not a settings file (a line that is not `key = value`, an unsupported kind of value). */
  | 'syntax'
  /** The same key twice. */
  | 'duplicate'
  /** A name neither the page nor the command line knows. */
  | 'unknown'
  /** A value of the wrong kind (text where a number is expected, ...). */
  | 'type'
  /** A number outside what the setting allows. */
  | 'range'
  /** A value that is not one of the allowed words, or not a valid color or list of sizes. */
  | 'value'
  /** A file far too large to be a settings file. */
  | 'size';

export interface Problem {
  code: ProblemCode;
  /** The key (or, for a syntax problem, the line's text). */
  key: string;
  /** The line, counted from 1. */
  line: number;
}

/** Why a file was not taken: every problem found, so they can be fixed in one go. */
export class SettingsFileError extends Error {
  constructor(readonly problems: Problem[]) {
    super(problems.map((problem) => `${problem.code} ${problem.key} (line ${problem.line})`).join('; '));
  }
}

export interface Imported {
  settings: Settings;
  /** Keys of the command line that the page has no use for: skipped. */
  skipped: string[];
  /** Sizes the page does not offer, left out of the list. */
  sizesLeftOut: number[];
  /** The padding was above what the page offers and was brought to it. */
  paddingLowered: boolean;
  /** The file says `sizes = "auto"` (the sizes the picture can supply): the page has no such setting, so the default sizes are used. */
  sizesAuto: boolean;
}

type Value = string | number | boolean | string[] | number[];
interface Entry {
  key: string;
  value: Value;
  line: number;
}

const KEY = /^[A-Za-z0-9_-]+$/;

function syntax(line: number, text: string): never {
  throw new SettingsFileError([{ code: 'syntax', key: text.trim().slice(0, 60), line }]);
}

/** A TOML string in quotes ("..." with escapes, '...' as it is), read from the start of `text`. */
function readString(text: string, line: number): [string, string] {
  const quote = text[0];
  if (quote === "'") {
    const end = text.indexOf("'", 1);
    if (end < 0) syntax(line, text);
    return [text.slice(1, end), text.slice(end + 1)];
  }
  let value = '';
  let i = 1;
  while (i < text.length) {
    const char = text[i];
    if (char === '"') return [value, text.slice(i + 1)];
    if (char === '\\') {
      const next = text[i + 1];
      const simple: Record<string, string> = { b: '\b', t: '\t', n: '\n', f: '\f', r: '\r', '"': '"', '\\': '\\' };
      if (next in simple) {
        value += simple[next];
        i += 2;
      } else if ((next === 'u' || next === 'U') && /^[0-9a-fA-F]+$/.test(text.slice(i + 2, i + 2 + (next === 'u' ? 4 : 8)))) {
        const digits = next === 'u' ? 4 : 8;
        value += String.fromCodePoint(parseInt(text.slice(i + 2, i + 2 + digits), 16));
        i += 2 + digits;
      } else {
        syntax(line, text);
      }
    } else {
      value += char;
      i += 1;
    }
  }
  return syntax(line, text);
}

/** One value (a string, a whole number, true or false) from the start of `text`, and what follows it. */
function readScalar(text: string, line: number): [string | number | boolean, string] {
  if (text[0] === '"' || text[0] === "'") return readString(text, line);
  const match = /^(true|false|[+-]?\d[\d_]*)(?![\w.:-])/.exec(text);
  if (!match) return syntax(line, text);
  const word = match[1];
  const rest = text.slice(word.length);
  if (word === 'true') return [true, rest];
  if (word === 'false') return [false, rest];
  return [Number(word.replace(/_/g, '')), rest];
}

/** The text after a value must be nothing, or a comment. */
function endOfLine(rest: string, line: number): void {
  const tail = rest.trim();
  if (tail !== '' && !tail.startsWith('#')) syntax(line, tail);
}

/**
 * Reads the flat TOML a settings file is: comments, `key = value` lines, values that are strings, whole
 * numbers, true/false, or lists of those (which may run over several lines). Tables, dates, decimals and
 * inline tables are not part of a settings file and are refused as a syntax problem.
 */
export function parseSettingsToml(text: string): Entry[] {
  const lines = text.replace(/^﻿/, '').split(/\r\n|\n|\r/);
  const entries: Entry[] = [];
  for (let at = 0; at < lines.length; at += 1) {
    const number = at + 1;
    const raw = lines[at].trim();
    if (raw === '' || raw.startsWith('#')) continue;
    const equals = raw.indexOf('=');
    if (equals < 0) syntax(number, raw);
    let key = raw.slice(0, equals).trim();
    if (key.length >= 2 && ((key[0] === '"' && key.endsWith('"')) || (key[0] === "'" && key.endsWith("'")))) key = key.slice(1, -1);
    if (!KEY.test(key)) syntax(number, raw);
    let rest = raw.slice(equals + 1).trim();
    let value: Value;
    if (rest.startsWith('[')) {
      // A list, possibly over several lines.
      const items: (string | number | boolean)[] = [];
      rest = rest.slice(1).trim();
      let line = number;
      for (;;) {
        if (rest === '' || rest.startsWith('#')) {
          at += 1;
          if (at >= lines.length) syntax(number, raw);
          line = at + 1;
          rest = lines[at].trim();
          continue;
        }
        if (rest.startsWith(']')) {
          rest = rest.slice(1);
          break;
        }
        const [item, after] = readScalar(rest, line);
        items.push(item);
        rest = after.trim();
        if (rest.startsWith(',')) rest = rest.slice(1).trim();
        else if (!rest.startsWith(']') && rest !== '' && !rest.startsWith('#')) syntax(line, rest);
      }
      endOfLine(rest, line);
      if (items.every((item) => typeof item === 'string')) value = items as string[];
      else if (items.every((item) => typeof item === 'number')) value = items as number[];
      else syntax(number, raw);
    } else {
      const [scalar, after] = readScalar(rest, number);
      endOfLine(after, number);
      value = scalar;
    }
    entries.push({ key, value, line: number });
  }
  return entries;
}

/** The sizes as the command line writes them: "16,32,48". */
function sizesText(sizes: number[]): string {
  return [...sizes].sort((a, b) => a - b).join(',');
}

function hexOf(color: string): string {
  return color.replace(/^#/, '').toUpperCase();
}

/**
 * The settings as a settings file. Only what the command line also knows is in it (the website package's names
 * and colors are not). The mirror, the turn, the crop and the GIF frame are in it when they are used: for the
 * command line they give the same icon, and the page takes them back.
 */
export function exportSettings(settings: Settings): string {
  const lines = [
    '# img2ico settings, saved from the web page.',
    '# For the command line:  img2ico --config img2ico.toml picture.png',
    '# (or leave the file named img2ico.toml in the folder you work in). On the page, use "Import settings".',
    '# It also holds the mirror, the turn, the crop and the GIF frame of the picture you worked on, when you used them.',
    '',
    `sizes = "${sizesText(settings.sizes)}"`,
    `padding = ${settings.padding}`,
    `corner-radius = ${settings.cornerRadius}`,
    `fit = "${settings.fit}"`,
    `grayscale = ${settings.grayscale}`,
    `trim = ${settings.trim}`,
  ];
  // The picture's own settings, in the order the command line applies them: mirror, turn, crop. The command line
  // counts the frames of a GIF from 1.
  if (settings.flipH) lines.push('flip-horizontal = true');
  if (settings.flipV) lines.push('flip-vertical = true');
  if (settings.rotate !== 0) lines.push(`rotate = ${settings.rotate}`);
  if (settings.crop) lines.push(`crop = "${settings.crop.x},${settings.crop.y},${settings.crop.width},${settings.crop.height}"`);
  if (settings.gifFrame > 0) lines.push(`gif-frame = ${settings.gifFrame + 1}`);
  if (settings.removeBackground) {
    lines.push(
      `chroma-key = "${settings.backgroundAuto ? 'auto' : hexOf(settings.backgroundColor)}"`,
      `tolerance = ${settings.tolerance}`,
      `feather = ${settings.feather}`,
    );
  }
  // The website package is a ZIP of several files; the command line has no setting for it.
  if (settings.format === 'ico' || settings.format === 'icns') lines.push(`output-format = "${settings.format}"`);
  return lines.join('\n') + '\n';
}

const wholeNumber = (value: Value): value is number => typeof value === 'number' && Number.isInteger(value);

/**
 * Settings out of the text of a settings file, laid over `current`: the website package's names and colors and the
 * file type (when the file has none) stay as they are; everything else the file does not say is the default (as it
 * is for the command line), also the mirror, the turn, the crop and the GIF frame. Throws a `SettingsFileError`
 * listing everything that is wrong, and changes nothing then.
 */
export function importSettings(text: string, current: Settings): Imported {
  const entries = parseSettingsToml(text);
  const problems: Problem[] = [];
  const seen = new Set<string>();
  const skipped: string[] = [];
  const result: Settings = {
    ...current,
    sizes: [...DEFAULT_SIZES],
    padding: 0,
    cornerRadius: 0,
    fit: 'contain',
    grayscale: false,
    trim: false,
    flipH: false,
    flipV: false,
    rotate: 0,
    crop: null,
    gifFrame: 0,
    removeBackground: false,
    backgroundAuto: true,
    backgroundColor: '#00ff00',
    tolerance: 20,
    feather: 50,
  };
  let sizesLeftOut: number[] = [];
  let paddingLowered = false;
  let sizesAuto = false;
  let sizesFromFile: number[] | null = null;
  let presetSizes: number[] | null = null;

  const bad = (entry: Entry, code: ProblemCode) => problems.push({ code, key: entry.key, line: entry.line });
  const number = (entry: Entry, min: number, max: number): number | null => {
    if (!wholeNumber(entry.value)) return bad(entry, 'type'), null;
    if (entry.value < min || entry.value > max) return bad(entry, 'range'), null;
    return entry.value;
  };
  const flag = (entry: Entry): boolean | null => (typeof entry.value === 'boolean' ? entry.value : (bad(entry, 'type'), null));
  const word = (entry: Entry): string | null => (typeof entry.value === 'string' ? entry.value : (bad(entry, 'type'), null));

  for (const entry of entries) {
    if (seen.has(entry.key)) {
      bad(entry, 'duplicate');
      continue;
    }
    seen.add(entry.key);
    if (!(CLI_KEYS as readonly string[]).includes(entry.key)) {
      bad(entry, 'unknown');
      continue;
    }
    switch (entry.key) {
      case 'sizes': {
        const text = word(entry);
        if (text === null) break;
        if (text.trim().toLowerCase() === 'auto') {
          sizesAuto = true;
          break;
        }
        // The command line takes any size; the page offers some of them (the others are reported).
        const sizes = text.split(',').map((part) => (/^\s*\d{1,6}\s*$/.test(part) ? Number(part) : NaN));
        if (sizes.some((size) => !(size >= 1))) bad(entry, 'value');
        else sizesFromFile = sizes;
        break;
      }
      case 'preset': {
        const name = word(entry);
        if (name === null) break;
        if (!(name in PRESET_SIZES)) bad(entry, 'value');
        else presetSizes = [...PRESET_SIZES[name]];
        break;
      }
      case 'chroma-key': {
        const text = word(entry);
        if (text === null) break;
        if (text.trim().toLowerCase() === 'auto') {
          result.removeBackground = true;
          result.backgroundAuto = true;
        } else if (/^#?[0-9a-fA-F]{6}$/.test(text.trim())) {
          result.removeBackground = true;
          result.backgroundAuto = false;
          result.backgroundColor = '#' + text.trim().replace(/^#/, '').toLowerCase();
        } else {
          bad(entry, 'value');
        }
        break;
      }
      case 'tolerance': {
        const value = number(entry, 0, 100);
        if (value !== null) result.tolerance = value;
        break;
      }
      case 'feather': {
        const value = number(entry, 0, 100);
        if (value !== null) result.feather = value;
        break;
      }
      case 'grayscale': {
        const value = flag(entry);
        if (value !== null) result.grayscale = value;
        break;
      }
      case 'trim': {
        const value = flag(entry);
        if (value !== null) result.trim = value;
        break;
      }
      case 'padding': {
        // The command line takes 0 to 100; the page offers up to 40.
        const value = number(entry, 0, 100);
        if (value !== null) {
          paddingLowered = value > 40;
          result.padding = Math.min(value, 40);
        }
        break;
      }
      case 'corner-radius': {
        const value = number(entry, 0, 50);
        if (value !== null) result.cornerRadius = value;
        break;
      }
      case 'flip-horizontal': {
        const value = flag(entry);
        if (value !== null) result.flipH = value;
        break;
      }
      case 'flip-vertical': {
        const value = flag(entry);
        if (value !== null) result.flipV = value;
        break;
      }
      case 'rotate': {
        // Any whole number of degrees, as on the command line; the page keeps it as 0 to 359.
        const value = number(entry, -1_000_000, 1_000_000);
        if (value !== null) result.rotate = ((value % 360) + 360) % 360;
        break;
      }
      case 'crop': {
        const text = word(entry);
        if (text === null) break;
        const parts = text.split(',').map((part) => (/^\s*\d{1,9}\s*$/.test(part) ? Number(part) : NaN));
        if (parts.length !== 4 || parts.some((part) => Number.isNaN(part)) || parts[2] < 1 || parts[3] < 1) bad(entry, 'value');
        else result.crop = { x: parts[0], y: parts[1], width: parts[2], height: parts[3] };
        break;
      }
      case 'gif-frame': {
        // The command line counts frames from 1, the page from 0.
        const value = number(entry, 1, 1_000_000);
        if (value !== null) result.gifFrame = value - 1;
        break;
      }
      case 'fit': {
        const name = word(entry);
        if (name === null) break;
        if (name === 'contain' || name === 'cover') result.fit = name as Fit;
        else bad(entry, 'value');
        break;
      }
      case 'output-format': {
        const name = word(entry);
        if (name === null) break;
        if (name === 'ico' || name === 'icns') result.format = name as Format;
        else bad(entry, 'value');
        break;
      }
      default:
        skipped.push(entry.key);
    }
  }
  if (problems.length > 0) throw new SettingsFileError(problems);

  // "preset" wins over "sizes", as it does on the command line.
  const chosen = presetSizes ?? sizesFromFile;
  if (chosen) {
    const offered = SIZE_CHOICES as readonly number[];
    const unique = [...new Set(chosen)];
    result.sizes = unique.filter((size) => offered.includes(size)).sort((a, b) => a - b);
    sizesLeftOut = unique.filter((size) => !offered.includes(size)).sort((a, b) => a - b);
    if (result.sizes.length === 0) {
      throw new SettingsFileError([{ code: 'value', key: presetSizes ? 'preset' : 'sizes', line: entries.find((e) => e.key === (presetSizes ? 'preset' : 'sizes'))?.line ?? 1 }]);
    }
  }
  return { settings: result, skipped, sizesLeftOut, paddingLowered, sizesAuto };
}
