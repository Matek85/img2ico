import { parseReport, type ValidationReport } from '../lib/report';
import type { EngineOptions } from '../lib/settings';
import type { Converted, Opened, Pixels, Request, Response } from './protocol';

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
      else waiting.reject(new Error(response.error));
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
  | { op: 'pixels'; bytes: Uint8Array; index: number }
  | { op: 'describe'; bytes: Uint8Array }
  | { op: 'extract'; bytes: Uint8Array; index: number }
  | { op: 'select'; bytes: Uint8Array; indices: number[] }
  | { op: 'merge'; files: { name: string; bytes: Uint8Array }[] }
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

function field<K extends 'value' | 'opened' | 'converted' | 'pixels' | 'data'>(
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

/** Merges two or more .ico files into one; images whose size was already there come back as warnings. */
export async function mergeIcons(files: { name: string; bytes: Uint8Array }[]): Promise<Converted> {
  return field(await call({ op: 'merge', files }), 'converted');
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

/** Lets the engine forget the open picture. */
export async function closePicture(): Promise<void> {
  await call({ op: 'close' });
}
