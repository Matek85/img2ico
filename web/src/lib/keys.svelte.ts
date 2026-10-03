// Whether the keyboard shortcuts are on (see shortcuts.ts); the choice is remembered in this browser.
import { loadShortcutsOn, saveShortcutsOn } from './storage';

export const keys = $state({ on: loadShortcutsOn() });

export function setShortcutsOn(on: boolean): void {
  keys.on = on;
  saveShortcutsOn(on);
}
