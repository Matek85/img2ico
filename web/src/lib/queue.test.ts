import { describe, expect, it } from 'vitest';
import { dropSlot, moveItem, placeAfterDrop, sizesText } from './queue';

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

describe('the sizes as text', () => {
  it('lists the sizes', () => {
    expect(sizesText([16, 32, 256])).toBe('16, 32, 256');
    expect(sizesText([])).toBe('');
  });
});

describe('dragging a row', () => {
  const middles = [10, 30, 50, 70];

  it('finds the slot under the pointer by the middles of the rows', () => {
    expect(dropSlot(middles, 0)).toBe(0);
    expect(dropSlot(middles, 20)).toBe(1);
    expect(dropSlot(middles, 49)).toBe(2);
    expect(dropSlot(middles, 500)).toBe(4);
  });

  it('puts the row where the slot says, counting without the row itself', () => {
    // Row 1 dragged below row 3 (slot 4): it ends up last, at index 3.
    expect(placeAfterDrop(1, 4)).toBe(3);
    // Row 3 dragged above row 1 (slot 1): index 1.
    expect(placeAfterDrop(3, 1)).toBe(1);
    // A slot next to the row itself changes nothing.
    expect(placeAfterDrop(2, 2)).toBe(2);
    expect(placeAfterDrop(2, 3)).toBe(2);
  });

  it('gives the same list when the row stays', () => {
    const list = ['a', 'b', 'c', 'd'];
    const from = 2;
    expect(moveItem(list, from, placeAfterDrop(from, dropSlot(middles, 45)) - from)).toEqual(list);
    expect(moveItem(list, from, placeAfterDrop(from, dropSlot(middles, 5)) - from)).toEqual(['c', 'a', 'b', 'd']);
  });
});
