import { describe, expect, it } from 'vitest';
import { shortName } from './names';

describe('shortening a file name', () => {
  it('leaves a name that fits alone', () => {
    expect(shortName('logo.png')).toBe('logo.png');
    expect(shortName('a'.repeat(32))).toBe('a'.repeat(32));
  });

  it('keeps the start and the file type of a long name', () => {
    const short = shortName('gadsijgklösdfjghsdklögjlöksdfjgölksdfjgölksdjhgölksdfjeioupr.ico', 24);
    expect(short).toHaveLength(24);
    expect(short.endsWith('….ico')).toBe(true);
    expect(short.startsWith('gadsijgklösdf')).toBe(true);
  });

  it('copes with no file type and with a type that is longer than it should be', () => {
    expect(shortName('x'.repeat(50), 10)).toBe('xxxxxxxxx…');
    expect(shortName('x'.repeat(50) + '.verylongtype', 10)).toHaveLength(10);
  });
});
