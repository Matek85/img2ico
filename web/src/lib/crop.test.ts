import { describe, expect, it } from 'vitest';
import {
  HANDLES,
  type Handle,
  MIN_EDGE,
  type Rect,
  aspectValue,
  clampRect,
  dragRect,
  fitAspect,
  fullRect,
  isFull,
} from './crop';

const bounds = { width: 200, height: 100 };

function inside(rect: Rect): boolean {
  return (
    Number.isInteger(rect.x) &&
    Number.isInteger(rect.y) &&
    Number.isInteger(rect.width) &&
    Number.isInteger(rect.height) &&
    rect.x >= 0 &&
    rect.y >= 0 &&
    rect.width >= 1 &&
    rect.height >= 1 &&
    rect.x + rect.width <= bounds.width &&
    rect.y + rect.height <= bounds.height
  );
}

describe('aspectValue', () => {
  it('turns a ratio into width over height', () => {
    expect(aspectValue('free')).toBeNull();
    expect(aspectValue('1:1')).toBe(1);
    expect(aspectValue('16:9')).toBeCloseTo(16 / 9);
  });
});

describe('clampRect', () => {
  it('keeps a frame inside the picture, in whole pixels', () => {
    expect(clampRect({ x: -5, y: 90.4, width: 300, height: 20 }, bounds)).toEqual({
      x: 0,
      y: 80,
      width: 200,
      height: 20,
    });
  });

  it('never makes an empty frame', () => {
    const rect = clampRect({ x: 5, y: 5, width: 0, height: -3 }, bounds);
    expect(rect.width).toBe(1);
    expect(rect.height).toBe(1);
  });
});

describe('isFull', () => {
  it('knows the whole picture', () => {
    expect(isFull(fullRect(bounds), bounds)).toBe(true);
    expect(isFull({ x: 0, y: 0, width: 199, height: 100 }, bounds)).toBe(false);
  });
});

describe('dragRect without a ratio', () => {
  const start: Rect = { x: 50, y: 20, width: 100, height: 40 };

  it('moves and stops at the edges of the picture', () => {
    expect(dragRect(start, 'move', 10, 5, bounds)).toEqual({ x: 60, y: 25, width: 100, height: 40 });
    expect(dragRect(start, 'move', 500, 500, bounds)).toEqual({ x: 100, y: 60, width: 100, height: 40 });
    expect(dragRect(start, 'move', -500, -500, bounds)).toEqual({ x: 0, y: 0, width: 100, height: 40 });
  });

  it('resizes from each side', () => {
    expect(dragRect(start, 'e', 20, 0, bounds)).toEqual({ x: 50, y: 20, width: 120, height: 40 });
    expect(dragRect(start, 'w', -20, 0, bounds)).toEqual({ x: 30, y: 20, width: 120, height: 40 });
    expect(dragRect(start, 's', 0, 10, bounds)).toEqual({ x: 50, y: 20, width: 100, height: 50 });
    expect(dragRect(start, 'n', 0, -10, bounds)).toEqual({ x: 50, y: 10, width: 100, height: 50 });
  });

  it('resizes from a corner', () => {
    expect(dragRect(start, 'se', 10, 10, bounds)).toEqual({ x: 50, y: 20, width: 110, height: 50 });
    expect(dragRect(start, 'nw', -10, -10, bounds)).toEqual({ x: 40, y: 10, width: 110, height: 50 });
  });

  it('cannot be dragged smaller than the minimum or through the other side', () => {
    const small = dragRect(start, 'e', -1000, 0, bounds);
    expect(small.width).toBe(MIN_EDGE);
    expect(small.x).toBe(50);
    const flipped = dragRect(start, 'w', 1000, 0, bounds);
    expect(flipped.width).toBe(MIN_EDGE);
    expect(flipped.x + flipped.width).toBe(150);
  });

  it('stays inside the picture whatever the drag', () => {
    for (const handle of ['move', ...HANDLES] as Handle[]) {
      for (const dx of [-1000, -37, 0, 41, 1000]) {
        for (const dy of [-1000, -13, 0, 29, 1000]) {
          const rect = dragRect(start, handle, dx, dy, bounds);
          expect(inside(rect), `${handle} ${dx},${dy}: ${JSON.stringify(rect)}`).toBe(true);
        }
      }
    }
  });
});

describe('dragRect with a ratio', () => {
  const aspect = 2;
  const start: Rect = { x: 50, y: 20, width: 80, height: 40 };

  it('keeps the ratio and the opposite corner when dragging a corner', () => {
    const rect = dragRect(start, 'se', 20, 0, bounds, aspect);
    expect(rect.x).toBe(50);
    expect(rect.y).toBe(20);
    expect(rect.width / rect.height).toBeCloseTo(aspect, 1);
    expect(rect.width).toBe(100);
  });

  it('keeps the ratio when dragging a side, around the centre of the other axis', () => {
    const rect = dragRect(start, 'e', 20, 0, bounds, aspect);
    expect(rect.width).toBe(100);
    expect(rect.height).toBe(50);
    expect(rect.x).toBe(50);
    expect(rect.y + rect.height / 2).toBeCloseTo(40, 0);
  });

  it('keeps the ratio and stays inside the picture whatever the drag', () => {
    for (const aspectRatio of [1, 4 / 3, 16 / 9, 0.5]) {
      const begin = fitAspect({ x: 30, y: 10, width: 120, height: 70 }, aspectRatio, bounds);
      for (const handle of HANDLES) {
        for (const dx of [-1000, -37, 0, 41, 1000]) {
          for (const dy of [-1000, -13, 0, 29, 1000]) {
            const rect = dragRect(begin, handle, dx, dy, bounds, aspectRatio);
            expect(inside(rect), `${handle} ${dx},${dy}: ${JSON.stringify(rect)}`).toBe(true);
            // Rounding to whole pixels allows a pixel of slack.
            expect(Math.abs(rect.width - rect.height * aspectRatio)).toBeLessThanOrEqual(
              aspectRatio + 1,
            );
          }
        }
      }
    }
  });
});

describe('fitAspect', () => {
  it('shrinks the frame to the ratio around its centre', () => {
    const rect = fitAspect({ x: 0, y: 0, width: 200, height: 100 }, 1, bounds);
    expect(rect).toEqual({ x: 50, y: 0, width: 100, height: 100 });
  });

  it('makes a wide frame of a tall one', () => {
    const rect = fitAspect({ x: 0, y: 0, width: 50, height: 100 }, 2, bounds);
    expect(rect).toEqual({ x: 0, y: 38, width: 50, height: 25 });
  });
});
