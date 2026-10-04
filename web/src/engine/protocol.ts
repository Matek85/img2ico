// The messages between the page and the engine worker.

import type { EngineOptions } from '../lib/settings';

/** A message of the engine: a code, the values in it (a value may be a message itself) and its English sentence. */
export interface EngineMessage {
  code: string;
  params: Record<string, string | EngineMessage>;
  text: string;
}

/** The website icon package the engine made. */
export interface FaviconPack {
  zip: Uint8Array;
  snippet: string;
  warnings: EngineMessage[];
}

export type Request =
  | { id: number; op: 'version' }
  | { id: number; op: 'validate'; bytes: Uint8Array }
  | { id: number; op: 'open'; bytes: Uint8Array; name: string; gifFrame: number }
  | { id: number; op: 'convert'; options: EngineOptions }
  | { id: number; op: 'convertOnce'; bytes: Uint8Array; name: string; gifFrame: number; jobs: EngineOptions[] }
  | { id: number; op: 'gifOpen'; bytes: Uint8Array; name: string }
  | { id: number; op: 'gifFrame'; index: number }
  | { id: number; op: 'gifSelect'; index: number }
  | { id: number; op: 'rotatedPreview'; degrees: number; flipH: boolean; flipV: boolean; maxEdge: number }
  | { id: number; op: 'pixels'; bytes: Uint8Array; index: number }
  | { id: number; op: 'describe'; bytes: Uint8Array }
  | { id: number; op: 'extract'; bytes: Uint8Array; index: number }
  | { id: number; op: 'select'; bytes: Uint8Array; indices: number[] }
  | { id: number; op: 'faviconPack'; options: EngineOptions; meta: { name: string; themeColor: string; appleBackground: string } }
  | { id: number; op: 'faviconSnippet'; hasSvg: boolean; themeColor: string }
  | { id: number; op: 'zipOpen'; bytes: Uint8Array }
  | { id: number; op: 'zipRead'; index: number }
  | { id: number; op: 'zipBuild'; files: { name: string; bytes: Uint8Array }[] }
  | { id: number; op: 'pngZip'; bytes: Uint8Array; stem: string }
  | { id: number; op: 'image'; options: EngineOptions; kind: string }
  | { id: number; op: 'iconImage'; bytes: Uint8Array; index: number; kind: string; background: string }
  | { id: number; op: 'iconImageZip'; bytes: Uint8Array; stem: string; kind: string; background: string }
  | { id: number; op: 'close' };

/** What `open` tells about the picture. */
export interface Opened {
  width: number;
  height: number;
  vector: boolean;
}

/** The frames of an animated GIF the engine holds. */
export interface GifInfo {
  count: number;
  width: number;
  height: number;
  /** How long each frame is shown, in milliseconds. */
  delays: number[];
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
  warnings: EngineMessage[];
}

export type Response =
  | { id: number; ok: true; value: string }
  | { id: number; ok: true; opened: Opened }
  | { id: number; ok: true; gif: GifInfo }
  | { id: number; ok: true; converted: Converted }
  | { id: number; ok: true; many: Converted[] }
  | { id: number; ok: true; pixels: Pixels }
  | { id: number; ok: true; data: Uint8Array }
  | { id: number; ok: true; pack: FaviconPack }
  | { id: number; ok: true }
  | { id: number; ok: false; error: string };
