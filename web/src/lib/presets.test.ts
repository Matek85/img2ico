import { describe, expect, it } from 'vitest';
import { STYLE_PRESETS, USE_PRESETS, WINDOWS_SIZES, isActive, withPreset } from './presets';
import { SIZE_CHOICES, defaultSettings } from './settings';

const preset = (list: typeof USE_PRESETS, id: string) => list.find((p) => p.id === id)!;

describe('presets', () => {
  it('only name sizes the page offers', () => {
    for (const p of USE_PRESETS) {
      for (const size of p.settings.sizes ?? []) {
        expect(SIZE_CHOICES as readonly number[]).toContain(size);
      }
    }
    expect(WINDOWS_SIZES).toHaveLength(10);
  });

  it('change only what they name', () => {
    const start = { ...defaultSettings(), grayscale: true, padding: 12 };
    const favicon = withPreset(start, preset(USE_PRESETS, 'favicon'));
    expect(favicon.sizes).toEqual([16, 32, 48]);
    expect(favicon.format).toBe('favicon');
    expect(favicon.grayscale).toBe(true);
    expect(favicon.padding).toBe(12);
    const rounded = withPreset(favicon, preset(STYLE_PRESETS, 'rounded'));
    expect(rounded.sizes).toEqual([16, 32, 48]);
    expect(rounded.cornerRadius).toBe(22);
  });

  it('do not share their size list with the settings', () => {
    const settings = withPreset(defaultSettings(), preset(USE_PRESETS, 'windows'));
    settings.sizes.push(1);
    expect(preset(USE_PRESETS, 'windows').settings.sizes).toHaveLength(10);
  });

  it('are recognized as active exactly while the settings match', () => {
    const favicon = preset(USE_PRESETS, 'favicon');
    const settings = withPreset(defaultSettings(), favicon);
    expect(isActive(settings, favicon)).toBe(true);
    expect(isActive({ ...settings, sizes: [48, 32, 16] }, favicon)).toBe(true);
    expect(isActive({ ...settings, sizes: [16, 32] }, favicon)).toBe(false);
    expect(isActive({ ...settings, format: 'icns' }, favicon)).toBe(false);
    expect(isActive(defaultSettings(), preset(STYLE_PRESETS, 'plain'))).toBe(true);
  });

  it('switch the file type with the macOS preset', () => {
    expect(withPreset(defaultSettings(), preset(USE_PRESETS, 'macos')).format).toBe('icns');
  });
});
