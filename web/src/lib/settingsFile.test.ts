import { readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import {
  CLI_KEYS,
  PRESET_SIZES,
  SettingsFileError,
  USED_KEYS,
  exportSettings,
  importSettings,
  parseSettingsToml,
} from './settingsFile';
import { type Settings, defaultSettings } from './settings';

const root = new URL('../../../', import.meta.url);
// (The page's type check has no Node types; this is only the environment of the test run.)
const env = (globalThis as unknown as { process: { env: Record<string, string | undefined> } }).process.env;
const read = (path: string) => readFileSync(new URL(path, root), 'utf8');

function problemsOf(text: string, current = defaultSettings()) {
  try {
    importSettings(text, current);
  } catch (error) {
    if (error instanceof SettingsFileError) return error.problems.map((p) => `${p.code}:${p.key}@${p.line}`);
    throw error;
  }
  return null;
}

describe('reading the file', () => {
  it('reads comments, strings, numbers, flags and lists (also over several lines)', () => {
    const entries = parseSettingsToml(
      [
        '# a comment',
        '',
        'sizes = "16,32"   # after a value',
        "fit = 'cover'",
        'padding = 1_0',
        'trim = true',
        '"corner-radius" = 5',
        'seeds = ["1,2", "3,4"]',
        'include = [',
        '  "*.png",   # one',
        '  "*.jpg",',
        ']',
        'quote = "a \\"b\\" \\u00e9\\n"',
      ].join('\n'),
    );
    expect(entries.map((entry) => [entry.key, entry.value])).toEqual([
      ['sizes', '16,32'],
      ['fit', 'cover'],
      ['padding', 10],
      ['trim', true],
      ['corner-radius', 5],
      ['seeds', ['1,2', '3,4']],
      ['include', ['*.png', '*.jpg']],
      ['quote', 'a "b" é\n'],
    ]);
    // Windows line ends and a byte order mark are no problem.
    expect(parseSettingsToml('﻿trim = true\r\nsizes = "16"\r\n')).toHaveLength(2);
  });

  it('refuses what is no settings file, naming the line', () => {
    for (const bad of ['[table]', 'sizes', 'sizes = ', 'sizes = 1.5', 'when = 1979-05-27', 'a = { b = 1 }', 'a = "open', 'a = [1, "x"]', 'a = 1 2', 'bad key = 1']) {
      expect(() => parseSettingsToml(`trim = true\n${bad}`), bad).toThrow(SettingsFileError);
    }
    try {
      parseSettingsToml('trim = true\n[table]');
    } catch (error) {
      expect((error as SettingsFileError).problems).toEqual([{ code: 'syntax', key: '[table]', line: 2 }]);
    }
  });
});

describe('writing the file', () => {
  it('writes the settings the command line knows, in its words', () => {
    const settings: Settings = { ...defaultSettings(), sizes: [48, 16, 32], padding: 10, cornerRadius: 20, fit: 'cover', grayscale: true };
    const lines = exportSettings(settings).split('\n').filter((line) => !line.startsWith('#') && line !== '');
    expect(lines).toEqual(['sizes = "16,32,48"', 'padding = 10', 'corner-radius = 20', 'fit = "cover"', 'grayscale = true', 'trim = false', 'output-format = "ico"']);
  });

  it('writes the background removal only when it is on, as "auto" or a color', () => {
    const base = { ...defaultSettings(), tolerance: 30, feather: 40 };
    expect(exportSettings({ ...base, removeBackground: true, backgroundAuto: true })).toContain('chroma-key = "auto"\ntolerance = 30\nfeather = 40');
    expect(exportSettings({ ...base, removeBackground: true, backgroundAuto: false, backgroundColor: '#12abEF' })).toContain('chroma-key = "12ABEF"');
    expect(exportSettings({ ...base, removeBackground: false })).not.toMatch(/chroma-key|tolerance|feather/);
  });

  it('writes the picture\'s own mirror, turn, crop and GIF frame when they are used, in the order the command line applies them', () => {
    const lines = exportSettings({
      ...defaultSettings(),
      flipH: true,
      flipV: true,
      rotate: 90,
      crop: { x: 1, y: 2, width: 30, height: 40 },
      gifFrame: 2,
    })
      .split('\n')
      .filter((line) => !line.startsWith('#') && line !== '');
    expect(lines.slice(5, 11)).toEqual([
      'trim = false',
      'flip-horizontal = true',
      'flip-vertical = true',
      'rotate = 90',
      'crop = "1,2,30,40"',
      // The command line counts the frames from 1, the page from 0.
      'gif-frame = 3',
    ]);
    // Not used: not written.
    const plain = exportSettings(defaultSettings());
    expect(plain).not.toMatch(/flip|rotate|crop =|gif-frame/);
  });

  it('leaves out the website package', () => {
    const text = exportSettings({ ...defaultSettings(), siteName: 'My site', themeColor: '#123456', format: 'favicon' });
    expect(text).not.toMatch(/My site|123456|favicon/);
    expect(text).not.toContain('output-format');
  });

  it('only uses keys the command line reads', () => {
    const settings = { ...defaultSettings(), removeBackground: true };
    for (const entry of parseSettingsToml(exportSettings(settings))) {
      expect(CLI_KEYS as readonly string[], entry.key).toContain(entry.key);
      expect(USED_KEYS as readonly string[], entry.key).toContain(entry.key);
    }
  });
});

describe('reading settings back', () => {
  const settings: Settings = {
    ...defaultSettings(),
    sizes: [16, 24, 32, 256],
    padding: 12,
    cornerRadius: 33,
    fit: 'cover',
    grayscale: true,
    trim: true,
    removeBackground: true,
    backgroundAuto: false,
    backgroundColor: '#12abef',
    tolerance: 55,
    feather: 7,
    format: 'icns',
    flipH: true,
    flipV: false,
    rotate: 30,
    crop: { x: 5, y: 6, width: 70, height: 80 },
    gifFrame: 4,
  };

  it('gives back what was written, also the picture\'s own mirror, turn, crop and GIF frame', () => {
    expect(importSettings(exportSettings(settings), defaultSettings()).settings).toEqual(settings);
    const auto = { ...settings, backgroundAuto: true, backgroundColor: defaultSettings().backgroundColor };
    expect(importSettings(exportSettings(auto), defaultSettings()).settings).toEqual(auto);
    const plain = defaultSettings();
    expect(importSettings(exportSettings(plain), plain).settings).toEqual(plain);
  });

  it('keeps the website package and the file type, and sets the rest as the file says (also the picture\'s own)', () => {
    const current: Settings = {
      ...defaultSettings(),
      padding: 30,
      grayscale: true,
      crop: { x: 1, y: 2, width: 3, height: 4 },
      rotate: 90,
      flipH: true,
      flipV: true,
      gifFrame: 3,
      siteName: 'Mine',
      themeColor: '#123456',
      appleBackground: '#abcdef',
      format: 'favicon',
    };
    const { settings: result } = importSettings('corner-radius = 10\n', current);
    // Not said in the file: the default (as for the command line), not what was there.
    expect(result.padding).toBe(0);
    expect(result.grayscale).toBe(false);
    expect(result.cornerRadius).toBe(10);
    expect(result).toMatchObject({ crop: null, rotate: 0, flipH: false, flipV: false, gifFrame: 0 });
    // The website package and the file type (not in the file) stay.
    expect(result).toMatchObject({ siteName: 'Mine', themeColor: '#123456', appleBackground: '#abcdef', format: 'favicon' });
  });

  it('reads hex colors with or without #, and "preset" wins over "sizes" as on the command line', () => {
    expect(importSettings('chroma-key = "#FFA500"', defaultSettings()).settings).toMatchObject({ removeBackground: true, backgroundAuto: false, backgroundColor: '#ffa500' });
    expect(importSettings('chroma-key = "ffa500"', defaultSettings()).settings.backgroundColor).toBe('#ffa500');
    expect(importSettings('sizes = "16,24"\npreset = "favicon"\n', defaultSettings()).settings.sizes).toEqual([16, 32, 48]);
    expect(importSettings('preset = "windows"\n', defaultSettings()).settings.sizes).toEqual([...PRESET_SIZES.windows]);
  });

  it('reads the mirror, the turn (any whole number of degrees, as 0 to 359), the crop and the GIF frame (counted from 1)', () => {
    const read = (text: string) => importSettings(text, defaultSettings()).settings;
    expect(read('flip-horizontal = true\nflip-vertical = true\n')).toMatchObject({ flipH: true, flipV: true });
    expect(read('rotate = -90\n').rotate).toBe(270);
    expect(read('rotate = 450\n').rotate).toBe(90);
    expect(read('rotate = 0\n').rotate).toBe(0);
    expect(read('crop = "10, 20, 300, 400"\n').crop).toEqual({ x: 10, y: 20, width: 300, height: 400 });
    expect(read('gif-frame = 1\n').gifFrame).toBe(0);
    expect(read('gif-frame = 5\n').gifFrame).toBe(4);
  });

  it('says what it skipped: settings of the command line only, sizes not offered, a padding brought down', () => {
    const imported = importSettings(
      ['jobs = 4', 'recursive = true', 'include = ["*.png"]', 'sizes = "16,18,20"'].join('\n'),
      defaultSettings(),
    );
    expect(imported.skipped).toEqual(['jobs', 'recursive', 'include']);
    expect(imported.sizesLeftOut).toEqual([18]);
    expect(imported.settings.sizes).toEqual([16, 20]);
    expect(importSettings('sizes = "auto"\n', defaultSettings())).toMatchObject({ sizesAuto: true, settings: { sizes: [...defaultSettings().sizes] } });
    const padded = importSettings('padding = 60\n', defaultSettings());
    expect(padded.paddingLowered).toBe(true);
    expect(padded.settings.padding).toBe(40);
  });

  it('refuses a name nobody knows, a value of the wrong kind or out of range, and says all of it at once', () => {
    expect(problemsOf('toleranse = 20\n')).toEqual(['unknown:toleranse@1']);
    expect(problemsOf('trim = true\ntrim = false\n')).toEqual(['duplicate:trim@2']);
    expect(problemsOf('tolerance = "high"\n')).toEqual(['type:tolerance@1']);
    expect(problemsOf('trim = 1\n')).toEqual(['type:trim@1']);
    expect(problemsOf('tolerance = 101\n')).toEqual(['range:tolerance@1']);
    expect(problemsOf('corner-radius = 51\n')).toEqual(['range:corner-radius@1']);
    expect(problemsOf('padding = 101\n')).toEqual(['range:padding@1']);
    expect(problemsOf('rotate = "left"\n')).toEqual(['type:rotate@1']);
    expect(problemsOf('rotate = 2000000\n')).toEqual(['range:rotate@1']);
    expect(problemsOf('flip-horizontal = "yes"\n')).toEqual(['type:flip-horizontal@1']);
    expect(problemsOf('gif-frame = 0\n')).toEqual(['range:gif-frame@1']);
    expect(problemsOf('crop = 5\n')).toEqual(['type:crop@1']);
    for (const bad of ['1,2,3', '1,2,3,0', '1,2,x,4', '-1,2,3,4', '1,2,3,4,5', '']) {
      expect(problemsOf(`crop = "${bad}"\n`), bad).toEqual(['value:crop@1']);
    }
    expect(problemsOf('fit = "stretch"\n')).toEqual(['value:fit@1']);
    expect(problemsOf('output-format = "png"\n')).toEqual(['value:output-format@1']);
    expect(problemsOf('chroma-key = "green"\n')).toEqual(['value:chroma-key@1']);
    expect(problemsOf('preset = "huge"\n')).toEqual(['value:preset@1']);
    expect(problemsOf('sizes = "16,x"\n')).toEqual(['value:sizes@1']);
    expect(problemsOf('sizes = "0,16"\n')).toEqual(['value:sizes@1']);
    expect(problemsOf('sizes = "16,512"\n')).toBeNull();
    expect(problemsOf('sizes = "512"\n')).toEqual(['value:sizes@1']);
    // Nothing offered is left: refused, not an empty list.
    expect(problemsOf('sizes = "18,72"\n')).toEqual(['value:sizes@1']);
    expect(problemsOf('sizes = "16,x"\ntolerance = 200\nnope = 1\n')).toEqual(['value:sizes@1', 'range:tolerance@2', 'unknown:nope@3']);
    expect(problemsOf('trim = true\n')).toBeNull();
  });
});

describe('the command line and the page agree', () => {
  it('knows exactly the keys the command line knows (KNOWN_SETTINGS_KEYS in src/config.rs)', () => {
    const source = read('src/config.rs');
    const list = /const KNOWN_SETTINGS_KEYS: &\[&str\] = &\[([^\]]*)\];/.exec(source)?.[1] ?? '';
    const keys = [...list.matchAll(/"([a-z-]+)"/g)].map((match) => match[1]);
    expect(keys.length).toBeGreaterThan(20);
    expect([...CLI_KEYS].sort()).toEqual([...keys].sort());
    for (const key of USED_KEYS) expect(keys, key).toContain(key);
  });

  it('has the sizes of the command line presets (SizePreset in src/cli.rs)', () => {
    const source = read('src/cli.rs');
    expect(source).toContain('SizePreset::Favicon => &[16, 32, 48]');
    expect(source).toContain('SizePreset::Minimal => &[16, 32]');
    const windows = /RECOMMENDED_WINDOWS_SIZES: \[u32; \d+\] = \[([^\]]*)\]/.exec(read('src/cli.rs'))?.[1] ?? '';
    expect(windows.split(',').map((n) => Number(n.trim())).filter(Boolean)).toEqual([...PRESET_SIZES.windows]);
  });

  it('reads every example settings file of the command line', () => {
    const dir = new URL('examples/', root);
    const files = readdirSync(dir).filter((name) => name.endsWith('.toml'));
    expect(files.length).toBeGreaterThanOrEqual(6);
    for (const name of files) {
      expect(() => importSettings(readFileSync(new URL(name, dir), 'utf8'), defaultSettings()), name).not.toThrow();
    }
  });

  // These are the files the page writes for some settings. The command line reads them (a test in tests/cli.rs).
  // To write them again after a change of the format: UPDATE_FIXTURES=1 npm test
  const fixtures: Record<string, Settings> = {
    'web-settings-plain.toml': { ...defaultSettings(), sizes: [16, 32, 48], padding: 10, cornerRadius: 20, fit: 'cover', grayscale: true },
    'web-settings-color.toml': { ...defaultSettings(), sizes: [32, 64, 128, 256], trim: true, removeBackground: true, backgroundAuto: false, backgroundColor: '#00ff00', tolerance: 30, feather: 40, format: 'icns' },
    'web-settings-auto.toml': { ...defaultSettings(), removeBackground: true, backgroundAuto: true },
    'web-settings-picture.toml': { ...defaultSettings(), sizes: [16, 32], flipH: true, rotate: 90, crop: { x: 4, y: 2, width: 20, height: 24 }, gifFrame: 2 },
  };
  for (const [name, settings] of Object.entries(fixtures)) {
    it(`writes ${name} as the file the command line test reads`, () => {
      const path = new URL(`tests/fixtures/${name}`, root);
      const text = exportSettings(settings);
      if (env.UPDATE_FIXTURES) writeFileSync(path, text);
      expect(read(`tests/fixtures/${name}`), 'run: UPDATE_FIXTURES=1 npm test (in web/)').toBe(text);
      expect(importSettings(text, defaultSettings()).settings).toEqual({ ...settings, backgroundColor: settings.removeBackground && !settings.backgroundAuto ? settings.backgroundColor : defaultSettings().backgroundColor });
    });
  }
});
