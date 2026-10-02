import { describe, expect, it } from 'vitest';
import { stemOf } from './download';

describe('stemOf', () => {
  it('drops the extension', () => {
    expect(stemOf('logo.ico')).toBe('logo');
    expect(stemOf('my.app.icon.ico')).toBe('my.app.icon');
  });

  it('copes with no extension and with nothing', () => {
    expect(stemOf('logo')).toBe('logo');
    expect(stemOf('')).toBe('icon');
    expect(stemOf('.ico')).toBe('.ico');
  });
});
