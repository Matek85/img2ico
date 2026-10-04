import { describe, expect, it } from 'vitest';
import { defaultSettings, downloadName, packMeta, toEngineOptions } from './settings';

describe('toEngineOptions', () => {
  it('sorts the sizes and leaves out the background unless it is on', () => {
    const settings = { ...defaultSettings(), sizes: [256, 16, 48] };
    const options = toEngineOptions(settings);
    expect(options.sizes).toEqual([16, 48, 256]);
    expect(options.background).toBeUndefined();
  });

  it('passes the background settings when removal is on', () => {
    const settings = {
      ...defaultSettings(),
      removeBackground: true,
      backgroundAuto: false,
      backgroundColor: '#336699',
    };
    expect(toEngineOptions(settings).background).toEqual({ spec: '#336699', tolerance: 20, feather: 50 });
    expect(toEngineOptions({ ...settings, backgroundAuto: true }).background?.spec).toBe('auto');
  });

  it('passes a crop only when there is one', () => {
    expect(toEngineOptions(defaultSettings()).crop).toBeUndefined();
    const settings = { ...defaultSettings(), crop: { x: 1, y: 2, width: 30, height: 40 } };
    expect(toEngineOptions(settings).crop).toEqual({ x: 1, y: 2, width: 30, height: 40 });
  });

  it('asks for the format it is given', () => {
    expect(toEngineOptions(defaultSettings(), 'icns').format).toBe('icns');
  });
});

describe('downloadName', () => {
  it('names an image after the picture and the type, or after the one size, or as a ZIP', () => {
    expect(downloadName('logo.png', 'image', { type: 'jpg', size: 'original', sizes: [16, 32] })).toBe('logo.jpg');
    expect(downloadName('logo.png', 'image', { type: 'webp', size: 'sizes', sizes: [48] })).toBe('logo_48x48.webp');
    expect(downloadName('logo.png', 'image', { type: 'tiff', size: 'sizes', sizes: [16, 32] })).toBe('logo_tiff.zip');
    expect(downloadName('logo.png', 'image')).toBe('logo.png');
  });

  it('replaces the extension', () => {
    expect(downloadName('logo.final.png', 'ico')).toBe('logo.final.ico');
    expect(downloadName('photo.jpeg', 'icns')).toBe('photo.icns');
  });

  it('names a website package a ZIP', () => {
    expect(downloadName('logo.png', 'favicon')).toBe('logo_favicon.zip');
    expect(downloadName('', 'favicon')).toBe('icon_favicon.zip');
    expect(packMeta({ ...defaultSettings(), siteName: 'Demo' })).toEqual({
      name: 'Demo',
      themeColor: '#ffffff',
      appleBackground: '#ffffff',
    });
  });

  it('copes with no extension or no name', () => {
    expect(downloadName('logo', 'ico')).toBe('logo.ico');
    expect(downloadName('.png', 'ico')).toBe('.png.ico');
    expect(downloadName('', 'ico')).toBe('icon.ico');
  });
});
