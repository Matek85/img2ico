// What the person can change, and how it becomes the options the engine
// understands (see `parse_options` in crates/wasm).

import type { Rect } from './crop';

export type Fit = 'contain' | 'cover';
export type Format = 'ico' | 'icns';

/** The sizes the page offers. An .ico can hold 1 to 256 pixels. */
export const SIZE_CHOICES = [16, 24, 32, 48, 64, 96, 128, 256] as const;

/** The sizes a Windows application icon normally has. */
export const DEFAULT_SIZES: readonly number[] = [16, 32, 48, 64, 128, 256];

export interface Settings {
  sizes: number[];
  padding: number;
  cornerRadius: number;
  fit: Fit;
  grayscale: boolean;
  trim: boolean;
  /** The part of the picture to use, in its pixels; `null` is all of it. */
  crop: Rect | null;
  removeBackground: boolean;
  /** Detect the background color from the picture's border. */
  backgroundAuto: boolean;
  /** The background color to remove when it is not detected, such as "#00ff00". */
  backgroundColor: string;
  tolerance: number;
  feather: number;
  format: Format;
}

export function defaultSettings(): Settings {
  return {
    sizes: [...DEFAULT_SIZES],
    padding: 0,
    cornerRadius: 0,
    fit: 'contain',
    grayscale: false,
    trim: false,
    crop: null,
    removeBackground: false,
    backgroundAuto: true,
    backgroundColor: '#00ff00',
    tolerance: 20,
    feather: 50,
    format: 'ico',
  };
}

/** The options as the engine takes them. */
export interface EngineOptions {
  format?: Format;
  sizes?: number[];
  padding?: number;
  cornerRadius?: number;
  fit?: Fit;
  grayscale?: boolean;
  trim?: boolean;
  crop?: Rect;
  background?: {
    spec: string;
    tolerance?: number;
    feather?: number;
  };
}

/**
 * The engine options for the preview or the download. The preview is always an
 * .ico of the chosen sizes (an .icns has its own fixed sizes, which would not
 * show what the person picked).
 */
export function toEngineOptions(settings: Settings, format: Format = 'ico'): EngineOptions {
  const options: EngineOptions = {
    format,
    sizes: [...settings.sizes].sort((a, b) => a - b),
    padding: settings.padding,
    cornerRadius: settings.cornerRadius,
    fit: settings.fit,
    grayscale: settings.grayscale,
    trim: settings.trim,
  };
  if (settings.crop) {
    options.crop = { ...settings.crop };
  }
  if (settings.removeBackground) {
    options.background = {
      spec: settings.backgroundAuto ? 'auto' : settings.backgroundColor,
      tolerance: settings.tolerance,
      feather: settings.feather,
    };
  }
  return options;
}

/** The name of the download: the picture's name with the icon's extension. */
export function downloadName(pictureName: string, format: Format): string {
  const dot = pictureName.lastIndexOf('.');
  const base = dot > 0 ? pictureName.slice(0, dot) : pictureName;
  return `${base || 'icon'}.${format}`;
}
