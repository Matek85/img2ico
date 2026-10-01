import { parseReport, type ValidationReport } from '../lib/report';
import type { Request, Response } from './protocol';

type Pending = { resolve: (value: string) => void; reject: (error: Error) => void };

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
      if (response.ok) waiting.resolve(response.value);
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

type Call = { op: 'version' } | { op: 'validate'; bytes: Uint8Array };

function call(request: Call, transfer: Transferable[] = []): Promise<string> {
  const id = nextId++;
  return new Promise((resolve, reject) => {
    pending.set(id, { resolve, reject });
    engine().postMessage({ ...request, id } satisfies Request, transfer);
  });
}

export function engineVersion(): Promise<string> {
  return call({ op: 'version' });
}

/** Checks the bytes of an .ico file. */
export async function validateIco(bytes: Uint8Array): Promise<ValidationReport> {
  return parseReport(await call({ op: 'validate', bytes }, [bytes.buffer]));
}
