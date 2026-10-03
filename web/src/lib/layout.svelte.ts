// What is printed on the keys of the visitor's keyboard. Where the browser can tell (Chromium: the Keyboard Map API,
// only in a secure context), the list of keys and the tooltips show the letter on the key for the shortcuts that
// are bound to a position (see shortcuts.ts); elsewhere they show the usual names with the known alternatives.
interface KeyboardLayoutMap {
  get(code: string): string | undefined;
}

const layout = $state<{ map: KeyboardLayoutMap | null }>({ map: null });

/** Asks the browser for the layout, once; without the API (or if it refuses) nothing changes. */
export function readLayout(): void {
  try {
    const keyboard = (navigator as Navigator & { keyboard?: { getLayoutMap?: () => Promise<KeyboardLayoutMap> } }).keyboard;
    keyboard?.getLayoutMap?.().then(
      (map) => (layout.map = map),
      () => {},
    );
  } catch {
    // No such API: the fallback names stay.
  }
}

/** The name printed on the key at the position `code` ("KeyZ"), or `fallback` where it is not known. */
export function keyName(code: string | undefined, fallback: string): string {
  const name = code ? layout.map?.get(code) : undefined;
  return name ? name.toUpperCase() : fallback;
}
