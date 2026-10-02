import { describe, expect, it } from 'vitest';
import { moveItem, sizesText } from './queue';

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
