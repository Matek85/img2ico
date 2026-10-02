// Picking a color out of the picture: where the pointer is in the picture, and
// the color as text.

/** `#rrggbb` for three channels 0-255. */
export function hexOf(r: number, g: number, b: number): string {
  return '#' + [r, g, b].map((v) => Math.max(0, Math.min(255, Math.round(v))).toString(16).padStart(2, '0')).join('');
}

export interface Box {
  left: number;
  top: number;
  width: number;
  height: number;
}

/**
 * The pixel of a picture under a pointer: `box` is where the picture is shown
 * on the page, `size` its pixels. Null outside the picture.
 */
export function pixelAt(box: Box, size: { width: number; height: number }, x: number, y: number): { x: number; y: number } | null {
  if (box.width <= 0 || box.height <= 0) return null;
  const fx = (x - box.left) / box.width;
  const fy = (y - box.top) / box.height;
  if (fx < 0 || fy < 0 || fx >= 1 || fy >= 1) return null;
  return { x: Math.floor(fx * size.width), y: Math.floor(fy * size.height) };
}
