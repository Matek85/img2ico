import { parseReport, type ValidationReport } from '../lib/report';
import type { EngineOptions } from '../lib/settings';
import type { Converted, Opened, Request, Response } from './protocol';

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
  | { op: 'close' };

function call(request: Call, transfer: Transferable[] = []): Promise<Response & { ok: true }> {
  const id = nextId++;
  return new Promise((resolve, reject) => {
    pending.set(id, { resolve, reject });
    engine().postMessage({ ...request, id } satisfies Request, transfer);
  });
}

function field<K extends 'value' | 'opened' | 'converted'>(
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

/** Lets the engine forget the open picture. */
export async function closePicture(): Promise<void> {
  await call({ op: 'close' });
}
