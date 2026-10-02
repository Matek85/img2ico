import { describe, expect, it } from 'vitest';
import { sanitize } from './storage';
import { defaultSettings } from './settings';

describe('sanitize', () => {
  it('gives the defaults for nothing or for rubbish', () => {
    expect(sanitize(null)).toEqual(defaultSettings());
    expect(sanitize('text')).toEqual(defaultSettings());
    expect(sanitize(42)).toEqual(defaultSettings());
  });

  it('keeps what is valid', () => {
    const stored = { ...defaultSettings(), sizes: [16, 48], padding: 12, grayscale: true, format: 'icns' };
    expect(sanitize(stored)).toEqual(stored);
  });

  it('replaces what is out of range or the wrong type, and keeps the rest', () => {
    const result = sanitize({ padding: 99, cornerRadius: '10', fit: 'stretch', format: 'png', tolerance: 5 });
    expect(result.padding).toBe(0);
    expect(result.cornerRadius).toBe(0);
    expect(result.fit).toBe('contain');
    expect(result.format).toBe('ico');
    expect(result.tolerance).toBe(5);
  });

  it('keeps only sizes the page offers, once each', () => {
    expect(sanitize({ sizes: [16, 16, 17, 'x', 256] }).sizes).toEqual([16, 256]);
    expect(sanitize({ sizes: 'nope' }).sizes).toEqual(defaultSettings().sizes);
    expect(sanitize({ sizes: [] }).sizes).toEqual([]);
  });

  it('keeps the website package settings and checks them', () => {
    const result = sanitize({ format: 'favicon', siteName: 'x'.repeat(100), themeColor: 'red', appleBackground: '#123456' });
    expect(result.format).toBe('favicon');
    expect(result.siteName).toHaveLength(60);
    expect(result.themeColor).toBe('#ffffff');
    expect(result.appleBackground).toBe('#123456');
  });

  it('never remembers a crop and checks the color', () => {
    expect(sanitize({ crop: { x: 1, y: 1, width: 5, height: 5 } }).crop).toBeNull();
    expect(sanitize({ backgroundColor: 'red' }).backgroundColor).toBe('#00ff00');
    expect(sanitize({ backgroundColor: '#ABCDEF' }).backgroundColor).toBe('#ABCDEF');
  });
});
