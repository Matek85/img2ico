import { describe, expect, it } from 'vitest';
import { cellAt, fitZoom, hexOf, moveCell, opacityPercent, pixelAt } from './pixels';

describe('pixelAt', () => {
  it('reads the pixel from row-major RGBA bytes', () => {
    const rgba = Uint8Array.from([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
    expect(pixelAt(rgba, 2, 1, 0)).toEqual({ r: 5, g: 6, b: 7, a: 8 });
    expect(pixelAt(rgba, 2, 0, 1)).toEqual({ r: 9, g: 10, b: 11, a: 12 });
  });
});

describe('hexOf and opacityPercent', () => {
  it('writes the alpha only when the pixel is not opaque', () => {
    expect(hexOf({ r: 255, g: 0, b: 16, a: 255 })).toBe('#ff0010');
    expect(hexOf({ r: 255, g: 0, b: 16, a: 128 })).toBe('#ff001080');
    expect(hexOf({ r: 0, g: 0, b: 0, a: 0 })).toBe('#00000000');
  });

  it('gives the opacity in percent', () => {
    expect(opacityPercent(255)).toBe('100');
    expect(opacityPercent(0)).toBe('0');
    expect(opacityPercent(128)).toBe('50.2');
  });
});

describe('fitZoom', () => {
  it('fills the width with whole pixels, at least 1 and at most the maximum', () => {
    expect(fitZoom(400, 16)).toBe(25);
    expect(fitZoom(400, 256)).toBe(1);
    expect(fitZoom(100, 256)).toBe(1);
    expect(fitZoom(10000, 2)).toBe(48);
    expect(fitZoom(0, 16)).toBe(1);
  });
});

describe('cellAt', () => {
  it('finds the pixel under a point, and nothing outside the image', () => {
    expect(cellAt(0, 0, 10, 4, 4)).toEqual({ x: 0, y: 0 });
    expect(cellAt(39, 25, 10, 4, 4)).toEqual({ x: 3, y: 2 });
    expect(cellAt(40, 0, 10, 4, 4)).toBeNull();
    expect(cellAt(-1, 5, 10, 4, 4)).toBeNull();
  });
});

describe('moveCell', () => {
  it('moves by arrows and stays inside the image', () => {
    expect(moveCell({ x: 1, y: 1 }, 'ArrowRight', 4, 4)).toEqual({ x: 2, y: 1 });
    expect(moveCell({ x: 3, y: 1 }, 'ArrowRight', 4, 4)).toEqual({ x: 3, y: 1 });
    expect(moveCell({ x: 0, y: 0 }, 'ArrowUp', 4, 4)).toEqual({ x: 0, y: 0 });
    expect(moveCell({ x: 2, y: 2 }, 'Home', 4, 4)).toEqual({ x: 0, y: 2 });
    expect(moveCell({ x: 2, y: 2 }, 'End', 4, 4)).toEqual({ x: 3, y: 2 });
    expect(moveCell({ x: 2, y: 2 }, 'a', 4, 4)).toEqual({ x: 2, y: 2 });
  });
});
