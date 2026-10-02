import { describe, expect, it } from 'vitest';
import { beforeLayout } from './compare';

describe('beforeLayout', () => {
  it('fits a wide picture into the square and centres it', () => {
    // 400 x 200 into 100: scale 0.25, drawn 100 x 50, 25 down from the top.
    expect(beforeLayout({ width: 400, height: 200 }, null, 100)).toEqual({
      width: 100,
      height: 50,
      x: 0,
      y: 25,
    });
  });

  it('fits a tall picture and a square one', () => {
    expect(beforeLayout({ width: 100, height: 200 }, null, 100)).toEqual({
      width: 50,
      height: 100,
      x: 25,
      y: 0,
    });
    expect(beforeLayout({ width: 64, height: 64 }, null, 128)).toEqual({
      width: 128,
      height: 128,
      x: 0,
      y: 0,
    });
  });

  it('shows only the cropped part, scaled to fill the square', () => {
    // The part (100, 50, 200 x 100) of a 400 x 200 picture, into a box of 100:
    // scale 0.5; the picture is drawn 200 x 100, shifted so the part is in view.
    expect(beforeLayout({ width: 400, height: 200 }, { x: 100, y: 50, width: 200, height: 100 }, 100)).toEqual({
      width: 200,
      height: 100,
      x: -50,
      y: 0,
    });
  });
});
