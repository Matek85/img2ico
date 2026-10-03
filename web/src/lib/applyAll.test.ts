import { describe, expect, it } from 'vitest';
import { PARTS, mergeSettings } from './applyAll';
import { defaultSettings } from './settings';

const target = {
  ...defaultSettings(),
  crop: { x: 1, y: 2, width: 30, height: 40 },
  rotate: 90,
  gifFrame: 3,
  sizes: [16],
  padding: 5,
  format: 'ico' as const,
  siteName: 'mine',
};
const source = {
  ...defaultSettings(),
  sizes: [32, 64],
  padding: 20,
  cornerRadius: 30,
  fit: 'cover' as const,
  grayscale: true,
  trim: true,
  removeBackground: true,
  backgroundAuto: false,
  backgroundColor: '#112233',
  tolerance: 55,
  feather: 10,
  format: 'icns' as const,
  siteName: 'other',
  crop: null,
};

describe('mergeSettings', () => {
  it('takes over only the chosen parts', () => {
    const sizes = mergeSettings(target, source, ['sizes']);
    expect(sizes.sizes).toEqual([32, 64]);
    expect(sizes.format).toBe('icns');
    expect(sizes.padding).toBe(5);
    expect(sizes.removeBackground).toBe(false);

    const look = mergeSettings(target, source, ['look']);
    expect(look).toMatchObject({ padding: 20, cornerRadius: 30, fit: 'cover', grayscale: true, trim: true, sizes: [16], format: 'ico' });

    const background = mergeSettings(target, source, ['background']);
    expect(background).toMatchObject({ removeBackground: true, backgroundAuto: false, backgroundColor: '#112233', tolerance: 55, feather: 10, padding: 5 });
  });

  it('never touches what belongs to one picture or to a website package', () => {
    const all = mergeSettings(target, source, PARTS);
    expect(all.crop).toEqual(target.crop);
    expect(all.rotate).toBe(90);
    expect(all.gifFrame).toBe(3);
    expect(all.siteName).toBe('mine');
  });

  it('changes nothing when nothing is chosen, and does not change the settings it is given', () => {
    expect(mergeSettings(target, source, [])).toEqual(target);
    const before = JSON.stringify(target);
    mergeSettings(target, source, PARTS);
    expect(JSON.stringify(target)).toBe(before);
    // The sizes are a copy, not shared.
    expect(mergeSettings(target, source, ['sizes']).sizes).not.toBe(source.sizes);
  });

  it('keeps the file type of an icon when the source is a website package', () => {
    expect(mergeSettings({ ...target, format: 'icns' }, { ...source, format: 'favicon' }, ['sizes']).format).toBe('icns');
  });
});
