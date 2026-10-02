// Remembering the settings between visits. They live in this browser only
// (localStorage) and are never sent anywhere. The crop frame is not kept: it
// belongs to one picture. Reading is defensive: whatever is stored is checked
// and anything unusable falls back to the defaults, so a damaged or outdated
// entry can never break the page.

import { SIZE_CHOICES, type Settings, defaultSettings } from './settings';

const KEY = 'img2ico.settings.v1';
const AUTOSAVE_KEY = 'img2ico.autosave.v1';

/** Whether changes to an icon of the queue are saved to it on their own (on unless it was switched off). */
export function loadAutoSave(): boolean {
  try {
    return localStorage.getItem(AUTOSAVE_KEY) !== '0';
  } catch {
    return true;
  }
}

export function saveAutoSave(on: boolean): void {
  try {
    localStorage.setItem(AUTOSAVE_KEY, on ? '1' : '0');
  } catch {
    // Without storage the choice only lasts until the page is closed.
  }
}

function whole(value: unknown, min: number, max: number, fallback: number): number {
  return typeof value === 'number' && Number.isInteger(value) && value >= min && value <= max
    ? value
    : fallback;
}

function color(value: unknown, fallback: string): string {
  return typeof value === 'string' && /^#[0-9a-fA-F]{6}$/.test(value) ? value : fallback;
}

/** The settings out of whatever was stored: valid parts are kept, the rest is the default. */
export function sanitize(raw: unknown): Settings {
  const base = defaultSettings();
  if (typeof raw !== 'object' || raw === null) return base;
  const data = raw as Record<string, unknown>;
  const offered = SIZE_CHOICES as readonly number[];
  const sizes = Array.isArray(data.sizes)
    ? data.sizes.filter((size): size is number => typeof size === 'number' && offered.includes(size))
    : base.sizes;
  return {
    sizes: Array.isArray(data.sizes) ? [...new Set(sizes)] : base.sizes,
    padding: whole(data.padding, 0, 40, base.padding),
    cornerRadius: whole(data.cornerRadius, 0, 50, base.cornerRadius),
    fit: data.fit === 'cover' ? 'cover' : 'contain',
    grayscale: data.grayscale === true,
    trim: data.trim === true,
    crop: null,
    removeBackground: data.removeBackground === true,
    backgroundAuto: data.backgroundAuto !== false,
    backgroundColor: color(data.backgroundColor, base.backgroundColor),
    tolerance: whole(data.tolerance, 0, 100, base.tolerance),
    feather: whole(data.feather, 0, 100, base.feather),
    format: data.format === 'icns' || data.format === 'favicon' ? data.format : 'ico',
    siteName: typeof data.siteName === 'string' ? data.siteName.slice(0, 60) : base.siteName,
    themeColor: color(data.themeColor, base.themeColor),
    appleBackground: color(data.appleBackground, base.appleBackground),
  };
}

/** The remembered settings, or the defaults. */
export function loadSettings(): Settings {
  try {
    const text = localStorage.getItem(KEY);
    return sanitize(text ? JSON.parse(text) : null);
  } catch {
    return defaultSettings();
  }
}

/** Remembers the settings; does nothing where storage is not available. */
export function saveSettings(settings: Settings): void {
  try {
    localStorage.setItem(KEY, JSON.stringify({ ...settings, crop: null }));
  } catch {
    // Private window, storage full or blocked: the page works without it.
  }
}
