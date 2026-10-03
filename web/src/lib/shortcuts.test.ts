import { describe, expect, it } from 'vitest';
import { type KeyInfo, SHORTCUTS, isTyping, shortcutOf, withKey } from './shortcuts';

function press(key: string, more: Partial<KeyInfo> = {}): KeyInfo {
  const code = /^\d$/.test(key) ? `Digit${key}` : /^[a-z]$/i.test(key) ? `Key${key.toUpperCase()}` : '';
  return { key, code, ctrlKey: false, metaKey: false, altKey: false, shiftKey: false, ...more };
}

describe('shortcutOf', () => {
  it('knows every listed shortcut by its label', () => {
    for (const shortcut of SHORTCUTS) {
      const info = press(shortcut.label.toLowerCase(), shortcut.label === '?' ? { shiftKey: true } : {});
      expect(shortcutOf(info), shortcut.label).toBe(shortcut.id);
    }
  });

  it('never takes a key pressed with Ctrl, Alt or Cmd (those are the browser\'s)', () => {
    for (const key of ['c', 'd', 's', 'r', 't', 'q', 'p', '1']) {
      expect(shortcutOf(press(key, { ctrlKey: true }))).toBeNull();
      expect(shortcutOf(press(key, { metaKey: true }))).toBeNull();
      expect(shortcutOf(press(key, { altKey: true }))).toBeNull();
    }
  });

  it('leaves Shift+letter alone, but takes the question mark', () => {
    expect(shortcutOf(press('c', { shiftKey: true }))).toBeNull();
    expect(shortcutOf(press('?', { shiftKey: true }))).toBe('help');
    expect(shortcutOf(press('?'))).toBe('help');
  });

  it('follows the typed letter, also in capitals (Caps Lock)', () => {
    expect(shortcutOf(press('D'))).toBe('download');
  });

  it('takes digits by the physical key, so a French keyboard works', () => {
    // AZERTY: the key of the digit 1 types "&" without Shift.
    expect(shortcutOf({ ...press('&'), code: 'Digit1' })).toBe('viewIcon');
    // The number pad is for placing the crop frame, not for the views.
    expect(shortcutOf({ ...press('1'), code: 'Numpad1' })).toBeNull();
    expect(shortcutOf({ ...press('!'), code: 'Digit1', shiftKey: true })).toBeNull();
  });

  it('ignores a held key, text composition and unknown keys', () => {
    expect(shortcutOf(press('d', { repeat: true }))).toBeNull();
    expect(shortcutOf(press('d', { isComposing: true }))).toBeNull();
    expect(shortcutOf(press('x'))).toBeNull();
    expect(shortcutOf(press('Escape'))).toBeNull();
    expect(shortcutOf(press('F5'))).toBeNull();
    expect(shortcutOf(press('4'))).toBeNull();
  });

  it('uses no key twice and none the browser takes without a modifier', () => {
    const labels = SHORTCUTS.map((s) => s.label);
    expect(new Set(labels).size).toBe(labels.length);
    // Firefox's "quick find" opens on / and ' ; Space and the arrows scroll or move.
    for (const taken of ['/', "'", ' ', 'ArrowLeft', 'ArrowRight']) expect(labels).not.toContain(taken);
  });
});

describe('isTyping', () => {
  const element = (tagName: string, extra: object = {}) => ({ tagName, ...extra }) as unknown as EventTarget;

  it('is true for text fields, text areas, lists and editable text', () => {
    expect(isTyping(element('INPUT', { type: 'text' }))).toBe(true);
    expect(isTyping(element('INPUT', { type: 'number' }))).toBe(true);
    expect(isTyping(element('TEXTAREA'))).toBe(true);
    expect(isTyping(element('SELECT'))).toBe(true);
    expect(isTyping(element('DIV', { isContentEditable: true }))).toBe(true);
  });

  it('is false for buttons, radios, checkboxes, sliders and colours (letters mean nothing there)', () => {
    for (const type of ['radio', 'checkbox', 'range', 'button', 'color']) {
      expect(isTyping(element('INPUT', { type }))).toBe(false);
    }
    expect(isTyping(element('BUTTON'))).toBe(false);
    expect(isTyping(element('BODY'))).toBe(false);
    expect(isTyping(null)).toBe(false);
  });
});

describe('withKey', () => {
  it('adds the key to a text while the shortcuts are on', () => {
    expect(withKey('Crop', 'crop', true)).toBe('Crop (C)');
    expect(withKey('Crop', 'crop', false)).toBe('Crop');
  });
});
