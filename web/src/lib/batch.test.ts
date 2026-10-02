import { describe, expect, it } from 'vitest';
import { baseName, isIconName, isPictureName, isZipName, outputName, stemOf } from './batch';

describe('names', () => {
  it('tells pictures, icons and ZIPs apart by extension, in either case', () => {
    expect(isPictureName('Logo.PNG')).toBe(true);
    expect(isPictureName('a/b/photo.jpeg')).toBe(true);
    expect(isPictureName('drawing.svg')).toBe(true);
    expect(isPictureName('notes.txt')).toBe(false);
    expect(isPictureName('README')).toBe(false);
    expect(isIconName('app.ICO')).toBe(true);
    expect(isIconName('app.png')).toBe(false);
    expect(isZipName('pack.zip')).toBe(true);
  });

  it('takes the last part of a path and drops the extension', () => {
    expect(baseName('art/icons/logo.png')).toBe('logo.png');
    expect(stemOf('art/icons/logo.final.png')).toBe('logo.final');
    expect(stemOf('')).toBe('icon');
  });
});

describe('outputName', () => {
  it('uses the picture name with the icon extension', () => {
    expect(outputName([], 'art/logo.png', 'ico')).toBe('logo.ico');
  });

  it('numbers a name that is already taken', () => {
    expect(outputName(['logo.ico'], 'a/logo.png', 'ico')).toBe('logo_2.ico');
    expect(outputName(['logo.ico', 'logo_2.ico'], 'b/logo.jpg', 'ico')).toBe('logo_3.ico');
    expect(outputName(['logo.ico'], 'x/other.png', 'icns')).toBe('other.icns');
  });
});
