import { EngineError } from '../lib/messages';
import { latestOnly } from '../lib/latest';
import { parseReport, type ValidationReport } from '../lib/report';
import type { EngineOptions } from '../lib/settings';
import type { Converted, FaviconPack, GifInfo, Opened, Pixels, Request, Response } from './protocol';

type Pending = { resolve: (response: Response & { ok: true }) => void; reject: (error: Error) => void };

let worker: Worker | undefined;
let nextId = 1;
const pending = new Map<number, Pending>();

function engine(): Worker {
  if (!worker) {
    worker = new Worker(new URL('./worker.ts', import.meta.url), { type: 'module' });
    worker.onmessage = (event: MessageEvent<Response>) => {
      const response = event.data;
      const waiting = pending.get(response.id);
      pending.delete(response.id);
      if (!waiting) return;
      if (response.ok) waiting.resolve(response);
      else waiting.reject(new EngineError(response.error));
    };
    worker.onerror = (event) => {
      for (const waiting of pending.values()) {
        waiting.reject(new Error(event.message || 'The engine stopped.'));
      }
      pending.clear();
      worker = undefined;
    };
  }
  return worker;
}

type Call =
  | { op: 'version' }
  | { op: 'validate'; bytes: Uint8Array }
  | { op: 'open'; bytes: Uint8Array; name: string; gifFrame: number }
  | { op: 'convert'; options: EngineOptions }
  | { op: 'gifOpen'; bytes: Uint8Array; name: string }
  | { op: 'gifFrame'; index: number }
  | { op: 'gifSelect'; index: number }
  | { op: 'pixels'; bytes: Uint8Array; index: number }
  | { op: 'describe'; bytes: Uint8Array }
  | { op: 'extract'; bytes: Uint8Array; index: number }
  | { op: 'select'; bytes: Uint8Array; indices: number[] }
  | { op: 'faviconPack'; options: EngineOptions; meta: { name: string; themeColor: string; appleBackground: string } }
  | { op: 'faviconSnippet'; hasSvg: boolean; themeColor: string }
  | { op: 'zipOpen'; bytes: Uint8Array }
  | { op: 'zipRead'; index: number }
  | { op: 'zipBuild'; files: { name: string; bytes: Uint8Array }[] }
  | { op: 'pngZip'; bytes: Uint8Array; stem: string }
  | { op: 'close' };

function call(request: Call, transfer: Transferable[] = []): Promise<Response & { ok: true }> {
  const id = nextId++;
  return new Promise((resolve, reject) => {
    pending.set(id, { resolve, reject });
    engine().postMessage({ ...request, id } satisfies Request, transfer);
  });
}

function field<K extends 'value' | 'opened' | 'gif' | 'converted' | 'pixels' | 'data' | 'pack'>(
  response: Response & { ok: true },
  key: K,
): NonNullable<Extract<Response, Record<K, unknown>>[K]> {
  if (!(key in response)) throw new Error('The engine returned an unexpected result.');
  return (response as unknown as Record<K, never>)[key];
}

export async function engineVersion(): Promise<string> {
  return field(await call({ op: 'version' }), 'value');
}

/** Checks the bytes of an .ico file. */
export async function validateIco(bytes: Uint8Array): Promise<ValidationReport> {
  return parseReport(field(await call({ op: 'validate', bytes }, [bytes.buffer]), 'value'));
}

/** Opens a picture in the engine; later `convert` calls work on it. */
export async function openPicture(bytes: Uint8Array, name: string, gifFrame = 1): Promise<Opened> {
  return field(await call({ op: 'open', bytes, name, gifFrame }, [bytes.buffer]), 'opened');
}

/** Makes the icon file the options ask for from the open picture. */
export async function convert(options: EngineOptions): Promise<Converted> {
  return field(await call({ op: 'convert', options }), 'converted');
}

/**
 * `convert` for the live preview: while one conversion runs, only the newest further request
 * is kept (the others reject with `Superseded`), so clicking through settings quickly does not
 * line up conversions of a big picture that nobody will look at.
 */
export const convertLatest = latestOnly(convert);

/** The exact pixels of image number `index` (from 0) of an .ico file. */
export async function iconPixels(bytes: Uint8Array, index: number): Promise<Pixels> {
  // The engine takes its own copy: the page keeps the file for the download.
  return field(await call({ op: 'pixels', bytes, index }), 'pixels');
}

/** What the engine says about the images of an .ico file. */
export interface IconDescription {
  images: {
    index: number;
    width: number;
    height: number;
    bits_per_pixel: number;
    format: 'PNG' | 'BMP';
    bytes: number;
    alpha: string;
    non_opaque_share: number | null;
  }[];
  missing_windows_sizes: number[];
}

/** The images of an .ico file, described. */
export async function describeIcon(bytes: Uint8Array): Promise<IconDescription> {
  return JSON.parse(field(await call({ op: 'describe', bytes }), 'value')) as IconDescription;
}

/** Image number `index` (from 0) of an .ico file, as a PNG file. */
export async function extractPng(bytes: Uint8Array, index: number): Promise<Uint8Array> {
  return field(await call({ op: 'extract', bytes, index }), 'data');
}

/** A new .ico file with only the images at `indices`. */
export async function selectImages(bytes: Uint8Array, indices: number[]): Promise<Uint8Array> {
  return field(await call({ op: 'select', bytes, indices }), 'data');
}

/** The website icon package (a ZIP) for the open picture, with the lines for the page's head. */
export async function faviconPack(
  options: EngineOptions,
  meta: { name: string; themeColor: string; appleBackground: string },
): Promise<FaviconPack> {
  return field(await call({ op: 'faviconPack', options, meta }), 'pack');
}

/** The lines for a page's head that go with the package. */
export async function faviconSnippet(hasSvg: boolean, themeColor: string): Promise<string> {
  return field(await call({ op: 'faviconSnippet', hasSvg, themeColor }), 'value');
}

/** A file inside a ZIP. */
export interface ZipFile {
  index: number;
  name: string;
  size: number;
}

/** Opens a ZIP and lists its files (folders and hidden files left out); `readZipFile` unpacks them. */
export async function openZip(bytes: Uint8Array): Promise<ZipFile[]> {
  return JSON.parse(field(await call({ op: 'zipOpen', bytes }, [bytes.buffer]), 'value')) as ZipFile[];
}

/** Unpacks one file of the ZIP opened last. */
export async function readZipFile(index: number): Promise<Uint8Array> {
  return field(await call({ op: 'zipRead', index }), 'data');
}

/** Writes files into a ZIP. */
export async function buildZip(files: { name: string; bytes: Uint8Array }[]): Promise<Uint8Array> {
  return field(await call({ op: 'zipBuild', files }), 'data');
}

/** Every image of an .ico file as a PNG, in a ZIP. */
export async function pngZip(bytes: Uint8Array, stem: string): Promise<Uint8Array> {
  return field(await call({ op: 'pngZip', bytes, stem }), 'data');
}

/** Reads all frames of an animated GIF into the engine (see `gifFrame`, `selectGifFrame`). */
export async function openGif(bytes: Uint8Array, name: string): Promise<GifInfo> {
  return field(await call({ op: 'gifOpen', bytes, name }, [bytes.buffer]), 'gif');
}

/** Frame number `index` (from 0) of the GIF read with `openGif`, as a PNG. */
export async function gifFramePng(index: number): Promise<Uint8Array> {
  return field(await call({ op: 'gifFrame', index }), 'data');
}

/** Makes frame `index` (from 0) the open picture, as if it had been opened on its own. */
export async function selectGifFrame(index: number): Promise<Opened> {
  return field(await call({ op: 'gifSelect', index }), 'opened');
}

/** Lets the engine forget the open picture. */
export async function closePicture(): Promise<void> {
  await call({ op: 'close' });
}
