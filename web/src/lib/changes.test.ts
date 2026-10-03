import { describe, expect, it } from 'vitest';
import { changedGroups } from './changes';
import { defaultSettings } from './settings';

describe('changedGroups', () => {
  it('names nothing when nothing changed, the GIF frame aside', () => {
    expect(changedGroups(defaultSettings(), { ...defaultSettings(), gifFrame: 3 })).toEqual([]);
  });

  it('names each group once, in the order of the settings', () => {
    const after = { ...defaultSettings(), padding: 10, flipH: true, flipV: true, tolerance: 40, removeBackground: true };
    expect(changedGroups(defaultSettings(), after)).toEqual(['padding', 'flip', 'background']);
  });

  it('sees a changed list and a changed crop frame', () => {
    const after = { ...defaultSettings(), sizes: [16], crop: { x: 1, y: 2, width: 3, height: 4 } };
    expect(changedGroups(defaultSettings(), after)).toEqual(['sizes', 'crop']);
  });
});
