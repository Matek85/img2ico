// Remembering the settings between visits. They live in this browser only
// (localStorage) and are never sent anywhere. The crop frame is not kept: it
// belongs to one picture. Reading is defensive: whatever is stored is checked
// and anything unusable falls back to the defaults, so a damaged or outdated
// entry can never break the page.

import { SIZE_CHOICES, type Settings, defaultSettings } from './settings';

const KEY = 'img2ico.settings.v1';

function whole(value: unknown, min: number, max: number, fallback: number): number {
  return typeof value === 'number' && Number.isInteger(value) && value >= min && value <= max
    ? value
    : fallback;
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
    backgroundColor:
      typeof data.backgroundColor === 'string' && /^#[0-9a-fA-F]{6}$/.test(data.backgroundColor)
        ? data.backgroundColor
        : base.backgroundColor,
    tolerance: whole(data.tolerance, 0, 100, base.tolerance),
    feather: whole(data.feather, 0, 100, base.feather),
    format: data.format === 'icns' ? 'icns' : 'ico',
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
