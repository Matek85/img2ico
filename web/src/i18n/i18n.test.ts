import { describe, expect, it } from 'vitest';
import { formatBytes, t } from './index';

describe('t', () => {
  it('fills in placeholders', () => {
    expect(t('result.file', { name: 'a.ico', size: '1 KB' })).toBe('a.ico, 1 KB');
  });

  it('picks the plural form from count', () => {
    expect(t('result.images', { count: 1 })).toBe('1 image');
    expect(t('result.images', { count: 3 })).toBe('3 images');
  });

  it('shows the key for an unknown text instead of nothing', () => {
    expect(t('no.such.key')).toBe('no.such.key');
  });

  it('keeps a placeholder that has no value', () => {
    expect(t('state.working', {})).toBe('Checking {name} …');
  });
});

describe('formatBytes', () => {
  it('uses the unit people expect', () => {
    expect(formatBytes(0)).toBe('0 B');
    expect(formatBytes(1023)).toBe('1,023 B');
    expect(formatBytes(1536)).toBe('1.5 KB');
    expect(formatBytes(5 * 1024 * 1024)).toBe('5 MB');
  });
});
