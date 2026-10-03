// What the person can change, and how it becomes the options the engine
// understands (see `parse_options` in crates/wasm).

import type { Rect } from './crop';

export type Fit = 'contain' | 'cover';
/** What the person wants to end up with: an .ico, an .icns, or a package for a website. */
export type Format = 'ico' | 'icns' | 'favicon';

/** The file types the engine writes. */
export type EngineFormat = 'ico' | 'icns' | 'png';

/** The sizes the page offers. An .ico can hold 1 to 256 pixels. */
export const SIZE_CHOICES = [16, 20, 24, 32, 40, 48, 64, 96, 128, 256] as const;

/** The sizes a Windows application icon normally has. */
export const DEFAULT_SIZES: readonly number[] = [16, 32, 48, 64, 128, 256];

export interface Settings {
  sizes: number[];
  padding: number;
  cornerRadius: number;
  fit: Fit;
  grayscale: boolean;
  trim: boolean;
  /** The picture is turned clockwise by this many degrees (0 to 359) before it is cropped. */
  rotate: number;
  /** The part of the (turned) picture to use, in its pixels; `null` is all of it. */
  crop: Rect | null;
  /** The frame of an animated GIF the icon is made from, counted from 0. */
  gifFrame: number;
  removeBackground: boolean;
  /** Detect the background color from the picture's border. */
  backgroundAuto: boolean;
  /** The background color to remove when it is not detected, such as "#00ff00". */
  backgroundColor: string;
  tolerance: number;
  feather: number;
  format: Format;
  /** For the website package: the site's name, and two colors. */
  siteName: string;
  themeColor: string;
  /** The Apple icon is laid on this color: an iPhone fills transparency with black. */
  appleBackground: string;
}

export function defaultSettings(): Settings {
  return {
    sizes: [...DEFAULT_SIZES],
    padding: 0,
    cornerRadius: 0,
    fit: 'contain',
    grayscale: false,
    trim: false,
    rotate: 0,
    crop: null,
    gifFrame: 0,
    removeBackground: false,
    backgroundAuto: true,
    backgroundColor: '#00ff00',
    tolerance: 20,
    feather: 50,
    format: 'ico',
    siteName: '',
    themeColor: '#ffffff',
    appleBackground: '#ffffff',
  };
}

/** The options as the engine takes them. */
export interface EngineOptions {
  format?: EngineFormat;
  sizes?: number[];
  padding?: number;
  cornerRadius?: number;
  fit?: Fit;
  grayscale?: boolean;
  trim?: boolean;
  rotate?: number;
  crop?: Rect;
  /** Lay the icon on this color (no transparency). */
  flatten?: string;
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
export function toEngineOptions(settings: Settings, format: EngineFormat = 'ico'): EngineOptions {
  const options: EngineOptions = {
    format,
    sizes: [...settings.sizes].sort((a, b) => a - b),
    padding: settings.padding,
    cornerRadius: settings.cornerRadius,
    fit: settings.fit,
    grayscale: settings.grayscale,
    trim: settings.trim,
  };
  if (settings.rotate) {
    options.rotate = settings.rotate;
  }
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
  const base = (dot > 0 ? pictureName.slice(0, dot) : pictureName) || 'icon';
  return format === 'favicon' ? `${base}_favicon.zip` : `${base}.${format}`;
}

/** The sizes of the favicon.ico in the website package. */
export const FAVICON_SIZES = [16, 32, 48];

/** The package's settings for the engine. */
export function packMeta(settings: Settings): { name: string; themeColor: string; appleBackground: string } {
  return {
    name: settings.siteName,
    themeColor: settings.themeColor,
    appleBackground: settings.appleBackground,
  };
}
