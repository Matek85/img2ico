// Using the settings of one icon of the queue for the others: what is taken over is chosen in parts. What belongs
// to one picture (the crop, the turn, the frame of a GIF) and the settings of a website package never are.
import type { Settings } from './settings';

export type Part = 'sizes' | 'look' | 'background';

/** The parts in the order they are offered. */
export const PARTS: readonly Part[] = ['sizes', 'look', 'background'];

/** `target` with the chosen parts of `source` laid over it. */
export function mergeSettings(target: Settings, source: Settings, parts: readonly Part[]): Settings {
  const merged: Settings = { ...target };
  if (parts.includes('sizes')) {
    merged.sizes = [...source.sizes];
    // A website package cannot be in the queue: an icon keeps the file type it has then.
    if (source.format === 'ico' || source.format === 'icns') merged.format = source.format;
  }
  if (parts.includes('look')) {
    merged.padding = source.padding;
    merged.cornerRadius = source.cornerRadius;
    merged.fit = source.fit;
    merged.grayscale = source.grayscale;
    merged.trim = source.trim;
  }
  if (parts.includes('background')) {
    merged.removeBackground = source.removeBackground;
    merged.backgroundAuto = source.backgroundAuto;
    merged.backgroundColor = source.backgroundColor;
    merged.tolerance = source.tolerance;
    merged.feather = source.feather;
  }
  return merged;
}
