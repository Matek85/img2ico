// Runs in a Web Worker: loads the WebAssembly engine once and answers the
// page's requests, so the page itself never stops to wait for it.

import init, { engine_version, validate_ico } from '../wasm/pkg/img2ico_wasm.js';
import type { Request, Response } from './protocol';

const ready = init();

self.onmessage = async (event: MessageEvent<Request>) => {
  const request = event.data;
  let response: Response;
  try {
    await ready;
    switch (request.op) {
      case 'version':
        response = { id: request.id, ok: true, value: engine_version() };
        break;
      case 'validate':
        response = { id: request.id, ok: true, value: validate_ico(request.bytes) };
        break;
    }
  } catch (error) {
    response = { id: request.id, ok: false, error: error instanceof Error ? error.message : String(error) };
  }
  self.postMessage(response);
};
