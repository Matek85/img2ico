import { describe, expect, it } from 'vitest';
import { hexOf, pixelAt } from './pick';

describe('color text', () => {
  it('is lower-case hex with two digits per channel', () => {
    expect(hexOf(0, 255, 16)).toBe('#00ff10');
    expect(hexOf(1, 2, 3)).toBe('#010203');
    expect(hexOf(300, -5, 127.6)).toBe('#ff0080');
  });
});

describe('the pixel under the pointer', () => {
  const box = { left: 100, top: 50, width: 200, height: 100 };
  const size = { width: 400, height: 200 };

  it('scales the position to the pixels of the picture', () => {
    expect(pixelAt(box, size, 100, 50)).toEqual({ x: 0, y: 0 });
    expect(pixelAt(box, size, 200, 100)).toEqual({ x: 200, y: 100 });
    expect(pixelAt(box, size, 299.9, 149.9)).toEqual({ x: 399, y: 199 });
  });

  it('is nothing outside the picture', () => {
    expect(pixelAt(box, size, 99, 60)).toBeNull();
    expect(pixelAt(box, size, 300, 60)).toBeNull();
    expect(pixelAt(box, size, 150, 150)).toBeNull();
    expect(pixelAt({ ...box, width: 0 }, size, 100, 50)).toBeNull();
  });
});
