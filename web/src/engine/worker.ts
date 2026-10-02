// Runs in a Web Worker: loads the WebAssembly engine once and answers the
// page's requests, so the page itself never stops to wait for it. It keeps the
// picture that was opened, so changing a setting converts it again without
// decoding it again.

import init, { Source, engine_version, validate_ico } from '../wasm/pkg/img2ico_wasm.js';
import type { Request, Response } from './protocol';

const ready = init();
// In a worker, postMessage takes the list of things to hand over (the DOM typings
// this project uses describe the page's version of it).
const scope = self as unknown as { postMessage(message: unknown, transfer: Transferable[]): void };
let source: Source | undefined;

function forget(): void {
  source?.free();
  source = undefined;
}

self.onmessage = async (event: MessageEvent<Request>) => {
  const request = event.data;
  let response: Response;
  const transfer: Transferable[] = [];
  try {
    await ready;
    switch (request.op) {
      case 'version':
        response = { id: request.id, ok: true, value: engine_version() };
        break;
      case 'validate':
        response = { id: request.id, ok: true, value: validate_ico(request.bytes) };
        break;
      case 'open': {
        forget();
        source = Source.open(request.bytes, request.name, request.gifFrame);
        response = {
          id: request.id,
          ok: true,
          opened: { width: source.width(), height: source.height(), vector: source.is_vector() },
        };
        break;
      }
      case 'convert': {
        if (!source) throw new Error('No picture is open.');
        const output = source.convert(JSON.stringify(request.options));
        const bytes = output.bytes();
        const converted = {
          bytes,
          sizes: Array.from(output.sizes()),
          warnings: JSON.parse(output.warnings()) as string[],
        };
        output.free();
        transfer.push(bytes.buffer);
        response = { id: request.id, ok: true, converted };
        break;
      }
      case 'close':
        forget();
        response = { id: request.id, ok: true };
        break;
    }
  } catch (error) {
    response = { id: request.id, ok: false, error: describe(error) };
  }
  scope.postMessage(response, transfer);
};

/** The text of an error: the engine's own errors are JavaScript errors with a message. */
function describe(error: unknown): string {
  if (error instanceof Error) return error.message;
  return String(error);
}
