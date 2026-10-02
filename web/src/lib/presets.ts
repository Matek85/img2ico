// Ready-made settings for the usual jobs. A preset changes only the settings
// it names and leaves the rest alone, so "Favicon" followed by "Rounded"
// gives a rounded favicon.

import { type Settings } from './settings';

/** The sizes Microsoft recommends, so Windows never has to stretch an icon. */
export const WINDOWS_SIZES = [16, 20, 24, 32, 40, 48, 64, 96, 128, 256];

export interface Preset {
  id: string;
  /** The settings it sets. */
  settings: Partial<Pick<Settings, 'sizes' | 'format' | 'padding' | 'cornerRadius' | 'fit'>>;
}

/** What the icon is for: the sizes and the file type. */
export const USE_PRESETS: Preset[] = [
  { id: 'windows', settings: { sizes: WINDOWS_SIZES, format: 'ico' } },
  { id: 'standard', settings: { sizes: [16, 32, 48, 64, 128, 256], format: 'ico' } },
  { id: 'favicon', settings: { sizes: [16, 32, 48], format: 'ico' } },
  { id: 'macos', settings: { sizes: [16, 32, 64, 128, 256], format: 'icns' } },
  { id: 'small', settings: { sizes: [16, 32], format: 'ico' } },
];

/** How the picture sits in its square. */
export const STYLE_PRESETS: Preset[] = [
  { id: 'plain', settings: { padding: 0, cornerRadius: 0, fit: 'contain' } },
  { id: 'rounded', settings: { padding: 6, cornerRadius: 22, fit: 'contain' } },
  { id: 'round', settings: { padding: 0, cornerRadius: 50, fit: 'cover' } },
];

function sameSizes(a: readonly number[], b: readonly number[]): boolean {
  return a.length === b.length && [...a].sort((x, y) => x - y).every((size, i) => size === [...b].sort((x, y) => x - y)[i]);
}

/** Whether the settings are exactly what the preset sets. */
export function isActive(settings: Settings, preset: Preset): boolean {
  const wanted = preset.settings;
  return (
    (wanted.sizes === undefined || sameSizes(settings.sizes, wanted.sizes)) &&
    (wanted.format === undefined || settings.format === wanted.format) &&
    (wanted.padding === undefined || settings.padding === wanted.padding) &&
    (wanted.cornerRadius === undefined || settings.cornerRadius === wanted.cornerRadius) &&
    (wanted.fit === undefined || settings.fit === wanted.fit)
  );
}

/** The settings with the preset applied. */
export function withPreset(settings: Settings, preset: Preset): Settings {
  return {
    ...settings,
    ...preset.settings,
    sizes: preset.settings.sizes ? [...preset.settings.sizes] : settings.sizes,
  };
}
