// Looking at single pixels of an icon image: which pixel is under the pointer,
// what its values are, and how big to draw the image.

export interface Pixel {
  r: number;
  g: number;
  b: number;
  a: number;
}

/** The pixel at (x, y) of an image of `width` pixels, from its RGBA bytes. */
export function pixelAt(rgba: Uint8Array, width: number, x: number, y: number): Pixel {
  const at = (y * width + x) * 4;
  return { r: rgba[at], g: rgba[at + 1], b: rgba[at + 2], a: rgba[at + 3] };
}

function hex2(value: number): string {
  return value.toString(16).padStart(2, '0');
}

/** "#rrggbb" - "#rrggbbaa" when the pixel is not fully opaque. */
export function hexOf(pixel: Pixel): string {
  const color = `#${hex2(pixel.r)}${hex2(pixel.g)}${hex2(pixel.b)}`;
  return pixel.a === 255 ? color : `${color}${hex2(pixel.a)}`;
}

/** The opacity as a percentage with at most one decimal: 128 is "50.2". */
export function opacityPercent(alpha: number): string {
  return String(Math.round((alpha / 255) * 1000) / 10);
}

/** The zoom (a whole number, at least 1) at which an image of `size` pixels fills `width`. */
export function fitZoom(width: number, size: number): number {
  if (size <= 0 || width <= 0) return 1;
  return Math.min(Math.max(Math.floor(width / size), 1), MAX_ZOOM);
}

export const MAX_ZOOM = 48;

/** The pixel (column, row) under a point at (`offsetX`, `offsetY`) of the drawn image, or `null` outside it. */
export function cellAt(
  offsetX: number,
  offsetY: number,
  zoom: number,
  width: number,
  height: number,
): { x: number; y: number } | null {
  const x = Math.floor(offsetX / zoom);
  const y = Math.floor(offsetY / zoom);
  if (x < 0 || y < 0 || x >= width || y >= height) return null;
  return { x, y };
}

/** The pixel after a key press: arrows move it by one, Home/End to the row's ends; stays inside. */
export function moveCell(
  cell: { x: number; y: number },
  key: string,
  width: number,
  height: number,
): { x: number; y: number } {
  const clamp = (value: number, max: number) => Math.min(Math.max(value, 0), max - 1);
  switch (key) {
    case 'ArrowLeft':
      return { x: clamp(cell.x - 1, width), y: cell.y };
    case 'ArrowRight':
      return { x: clamp(cell.x + 1, width), y: cell.y };
    case 'ArrowUp':
      return { x: cell.x, y: clamp(cell.y - 1, height) };
    case 'ArrowDown':
      return { x: cell.x, y: clamp(cell.y + 1, height) };
    case 'Home':
      return { x: 0, y: cell.y };
    case 'End':
      return { x: width - 1, y: cell.y };
    default:
      return cell;
  }
}
