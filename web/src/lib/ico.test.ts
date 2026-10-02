import { describe, expect, it } from 'vitest';
import { iconEntries } from './ico';

/** An .ico with one PNG-looking image of the given size (0 means 256). */
function ico(images: { width: number; data: number[] }[]): Uint8Array {
  const header = [0, 0, 1, 0, images.length, 0];
  let offset = 6 + images.length * 16;
  const directory: number[] = [];
  const payload: number[] = [];
  for (const { width, data } of images) {
    const entry = new Uint8Array(16);
    const view = new DataView(entry.buffer);
    entry[0] = width;
    entry[1] = width;
    view.setUint32(8, data.length, true);
    view.setUint32(12, offset, true);
    directory.push(...entry);
    payload.push(...data);
    offset += data.length;
  }
  return Uint8Array.from([...header, ...directory, ...payload]);
}

const PNG = [0x89, 0x50, 0x4e, 0x47, 1, 2, 3];

describe('iconEntries', () => {
  it('finds the images and their sizes, 0 meaning 256', () => {
    const entries = iconEntries(ico([{ width: 16, data: PNG }, { width: 0, data: PNG }]));
    expect(entries.map((e) => e.size)).toEqual([16, 256]);
    expect(Array.from(entries[0].png)).toEqual(PNG);
  });

  it('skips an image that is not a PNG', () => {
    expect(iconEntries(ico([{ width: 16, data: [1, 2, 3, 4] }]))).toEqual([]);
  });

  it('gives nothing for something that is not an .ico or is cut off', () => {
    expect(iconEntries(new Uint8Array([1, 2, 3]))).toEqual([]);
    expect(iconEntries(ico([{ width: 16, data: PNG }]).subarray(0, 25))).toEqual([]);
  });
});
