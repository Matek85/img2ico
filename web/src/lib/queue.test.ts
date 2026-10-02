import { describe, expect, it } from 'vitest';
import { canCombine, moveItem, overlaps, sizesText } from './queue';

describe('moving an item of the queue', () => {
  it('moves it up or down by one place', () => {
    expect(moveItem(['a', 'b', 'c'], 1, -1)).toEqual(['b', 'a', 'c']);
    expect(moveItem(['a', 'b', 'c'], 1, 1)).toEqual(['a', 'c', 'b']);
  });

  it('leaves the list as it is at the ends, and does not change the original', () => {
    const list = ['a', 'b', 'c'];
    expect(moveItem(list, 0, -1)).toEqual(['a', 'b', 'c']);
    expect(moveItem(list, 2, 1)).toEqual(['a', 'b', 'c']);
    expect(moveItem(list, 5, -1)).toEqual(['a', 'b', 'c']);
    expect(moveItem(list, 0, 1)).toEqual(['b', 'a', 'c']);
    expect(list).toEqual(['a', 'b', 'c']);
  });
});

describe('combining icons', () => {
  it('needs two or more .ico files', () => {
    expect(canCombine([])).toBe(false);
    expect(canCombine(['ico'])).toBe(false);
    expect(canCombine(['ico', 'ico'])).toBe(true);
    expect(canCombine(['ico', 'icns'])).toBe(false);
  });

  it('lists the sizes', () => {
    expect(sizesText([16, 32, 256])).toBe('16, 32, 256');
    expect(sizesText([])).toBe('');
  });
});

describe('what combining leaves out', () => {
  it('lists, from the second icon on, the sizes already there', () => {
    const found = overlaps([
      { name: 'a.ico', sizes: [16, 32, 48] },
      { name: 'b.ico', sizes: [32, 64] },
      { name: 'c.ico', sizes: [16, 32, 48, 64] },
    ]);
    expect(found).toEqual([
      { name: 'b.ico', left: [32], total: 2 },
      { name: 'c.ico', left: [16, 32, 48, 64], total: 4 },
    ]);
  });

  it('has nothing to say when the sizes do not collide, or for the first icon alone', () => {
    expect(overlaps([{ name: 'a.ico', sizes: [16, 32] }, { name: 'b.ico', sizes: [48, 256] }])).toEqual([]);
    expect(overlaps([{ name: 'a.ico', sizes: [16, 16, 32] }])).toEqual([]);
    expect(overlaps([])).toEqual([]);
  });
});
