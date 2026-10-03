// The keyboard shortcuts of the editor.
//
// They are single keys without Ctrl, Alt or Cmd, so none of them can clash with a shortcut of
// the browser or the system (Ctrl+S, Ctrl+D, Alt+Left, ...), and a combination the page does
// not know is left to the browser. They are not active while a text is being typed, and they
// can be switched off (a single key that cannot be turned off is a problem for people who
// dictate or use assistive technology: WCAG 2.1.4).
//
// Letters are matched by the character typed (`key`), so they follow the keyboard layout.
// Digits are matched by the physical key (`code`): on a French keyboard the digit needs Shift,
// and the key on the left of the row would otherwise never be a 1. Only the row above the letters
// counts: the number pad belongs to the crop, where it places the frame like a map (7 8 9 / 4 5 6 / 1 2 3).
// Escape (ends the crop) is handled in the editor; the keys of the crop (Space, arrows, plus and
// minus) are handled by the crop itself, and CROP_KEYS only lists them.

export type ShortcutId =
  | 'viewIcon'
  | 'viewCompare'
  | 'viewPixels'
  | 'crop'
  | 'reset'
  | 'settings'
  | 'download'
  | 'queueAdd'
  | 'theme'
  | 'help'
  | 'play'
  | 'framePrev'
  | 'frameNext';

export type ShortcutGroup = 'view' | 'edit' | 'save' | 'page' | 'gif';

export interface Shortcut {
  id: ShortcutId;
  /** The key as the page shows it. */
  label: string;
  group: ShortcutGroup;
}

/** Every shortcut, in the order they are listed. */
export const SHORTCUTS: readonly Shortcut[] = [
  { id: 'viewIcon', label: '1', group: 'view' },
  { id: 'viewCompare', label: '2', group: 'view' },
  { id: 'viewPixels', label: '3', group: 'view' },
  { id: 'crop', label: 'C', group: 'edit' },
  { id: 'reset', label: 'R', group: 'edit' },
  { id: 'settings', label: 'S', group: 'edit' },
  { id: 'download', label: 'D', group: 'save' },
  { id: 'queueAdd', label: 'Q', group: 'save' },
  { id: 'play', label: 'P', group: 'gif' },
  { id: 'framePrev', label: ',', group: 'gif' },
  { id: 'frameNext', label: '.', group: 'gif' },
  { id: 'theme', label: 'T', group: 'page' },
  { id: 'help', label: '?', group: 'page' },
];

/** The keys of the crop (handled by the crop itself, see CropTool.svelte), for the list. */
export const CROP_KEYS: readonly { label: string; action: string; letter?: string; also?: string; labelKey?: string }[] = [
  { label: 'Space', labelKey: 'keys.space', action: 'lock' },
  { label: '← ↑ → ↓', action: 'move' },
  { label: '+ −', action: 'zoom' },
  { label: 'Num 1–9', labelKey: 'keys.num_range', action: 'place' },
  // Y does the same as Z: on a German keyboard (QWERTZ) the key beside X is the Y.
  { label: 'Z / Y', action: 'turnLeft', letter: 'z', also: 'y' },
  { label: 'X', action: 'turnRight', letter: 'x' },
  { label: 'M', action: 'shape', letter: 'm' },
  { label: 'F', action: 'grow', letter: 'f' },
  { label: 'O', action: 'rotate', letter: 'o' },
  { label: 'Esc', action: 'done' },
];

export const GROUPS: readonly ShortcutGroup[] = ['view', 'edit', 'save', 'gif', 'page'];

const BY_LETTER: Record<string, ShortcutId> = {
  c: 'crop',
  r: 'reset',
  s: 'settings',
  d: 'download',
  q: 'queueAdd',
  t: 'theme',
  p: 'play',
  ',': 'framePrev',
  '.': 'frameNext',
  '?': 'help',
};

const BY_DIGIT: Record<string, ShortcutId> = {
  '1': 'viewIcon',
  '2': 'viewCompare',
  '3': 'viewPixels',
};

/** The part of a keyboard event that decides. */
export interface KeyInfo {
  key: string;
  code: string;
  ctrlKey: boolean;
  metaKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  isComposing?: boolean;
  repeat?: boolean;
}

/** Which shortcut a key press is, or null (a modifier is held, the key is held down, or it is none of ours). */
export function shortcutOf(event: KeyInfo): ShortcutId | null {
  if (event.ctrlKey || event.metaKey || event.altKey || event.isComposing || event.repeat) return null;
  const digit = /^Digit([123])$/.exec(event.code);
  if (digit) return event.shiftKey ? null : BY_DIGIT[digit[1]];
  // "?" needs Shift on most keyboards; for any other key Shift means it is not ours.
  if (event.key === '?') return 'help';
  if (event.shiftKey) return null;
  return BY_LETTER[event.key.toLowerCase()] ?? null;
}

const NOT_TEXT = new Set(['checkbox', 'radio', 'range', 'button', 'submit', 'reset', 'color', 'file', 'image']);

/** Whether keys typed at this element are text (or choose an entry of a list) and so are not shortcuts. */
export function isTyping(target: EventTarget | null): boolean {
  const element = target as Partial<HTMLElement> | null;
  if (!element || typeof element.tagName !== 'string') return false;
  if (element.isContentEditable) return true;
  const tag = element.tagName;
  if (tag === 'TEXTAREA' || tag === 'SELECT') return true;
  if (tag === 'INPUT') return !NOT_TEXT.has(((element as HTMLInputElement).type ?? 'text').toLowerCase());
  return false;
}

/** The text of a control with its key, like "Crop (C)"; the text alone where the shortcuts are off. */
export function withKey(text: string, id: ShortcutId, on: boolean): string {
  const shortcut = SHORTCUTS.find((s) => s.id === id);
  return on && shortcut ? `${text} (${shortcut.label})` : text;
}
