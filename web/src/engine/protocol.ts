// The messages between the page and the engine worker.

import type { EngineOptions } from '../lib/settings';

export type Request =
  | { id: number; op: 'version' }
  | { id: number; op: 'validate'; bytes: Uint8Array }
  | { id: number; op: 'open'; bytes: Uint8Array; name: string; gifFrame: number }
  | { id: number; op: 'convert'; options: EngineOptions }
  | { id: number; op: 'pixels'; bytes: Uint8Array; index: number }
  | { id: number; op: 'close' };

/** What `open` tells about the picture. */
export interface Opened {
  width: number;
  height: number;
  vector: boolean;
}

/** One image of an icon file, as exact pixels. */
export interface Pixels {
  width: number;
  height: number;
  /** Four bytes a pixel (red, green, blue, alpha), row by row from the top left. */
  rgba: Uint8Array;
}

/** What `convert` returns: the icon file, the sizes in it and the warnings. */
export interface Converted {
  bytes: Uint8Array;
  sizes: number[];
  warnings: string[];
}

export type Response =
  | { id: number; ok: true; value: string }
  | { id: number; ok: true; opened: Opened }
  | { id: number; ok: true; converted: Converted }
  | { id: number; ok: true; pixels: Pixels }
  | { id: number; ok: true }
  | { id: number; ok: false; error: string };
