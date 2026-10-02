// The "before" side of the before/after comparison: the original picture (or
// the part of it the crop frame covers) laid into a square, the way the icon is
// laid into its square - the whole of it visible, centred. It is drawn with
// CSS from the original file, so no extra conversion is needed.

import type { Rect, Size } from './crop';

export interface BackgroundLayout {
  /** The size the whole original is drawn at, in the units of `box`. */
  width: number;
  height: number;
  /** Where its top left corner is, relative to the square's. */
  x: number;
  y: number;
}

/**
 * How to draw `picture` so that its `crop` part (all of it if `crop` is null)
 * fits into a square of edge `box`, centred.
 */
export function beforeLayout(picture: Size, crop: Rect | null, box: number): BackgroundLayout {
  const part = crop ?? { x: 0, y: 0, width: picture.width, height: picture.height };
  const scale = box / Math.max(part.width, part.height);
  return {
    width: picture.width * scale,
    height: picture.height * scale,
    x: (box - part.width * scale) / 2 - part.x * scale,
    y: (box - part.height * scale) / 2 - part.y * scale,
  };
}
