// Runs in a Web Worker: loads the WebAssembly engine once and answers the
// page's requests, so the page itself never stops to wait for it. It keeps the
// picture that was opened, so changing a setting converts it again without
// decoding it again.

import init, {
  GifFrames,
  Source,
  ZipBuilder,
  ZipReader,
  engine_version,
  favicon_snippet,
  icon_describe,
  icon_extract_png,
  icon_pixels,
  icon_png_zip,
  icon_select,
  validate_ico,
} from '../wasm/pkg/img2ico_wasm.js';
import type { EngineMessage, Request, Response } from './protocol';

const ready = init();
// In a worker, postMessage takes the list of things to hand over (the DOM typings
// this project uses describe the page's version of it).
const scope = self as unknown as { postMessage(message: unknown, transfer: Transferable[]): void };
let source: Source | undefined;
// The ZIP whose files are being read one by one.
let zip: ZipReader | undefined;
// The frames of an animated GIF, to show them and to choose one.
let gif: GifFrames | undefined;

function forget(): void {
  source?.free();
  source = undefined;
}

function forgetGif(): void {
  gif?.free();
  gif = undefined;
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
        forgetGif();
        source = Source.open(request.bytes, request.name, request.gifFrame);
        response = {
          id: request.id,
          ok: true,
          opened: { width: source.width(), height: source.height(), vector: source.is_vector() },
        };
        break;
      }
      case 'gifOpen': {
        forgetGif();
        gif = GifFrames.open(request.bytes, request.name);
        const delays = Array.from({ length: gif.count() }, (_, index) => gif!.delay_ms(index));
        response = { id: request.id, ok: true, gif: { count: gif.count(), width: gif.width(), height: gif.height(), delays } };
        break;
      }
      case 'gifFrame': {
        if (!gif) throw new Error('No GIF is open.');
        const data = gif.frame_png(request.index);
        transfer.push(data.buffer);
        response = { id: request.id, ok: true, data };
        break;
      }
      case 'gifSelect': {
        if (!gif) throw new Error('No GIF is open.');
        const chosen = Source.from_gif_frame(gif, request.index);
        forget();
        source = chosen;
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
          warnings: JSON.parse(output.warnings()) as EngineMessage[],
        };
        output.free();
        transfer.push(bytes.buffer);
        response = { id: request.id, ok: true, converted };
        break;
      }
      case 'pixels': {
        const image = icon_pixels(request.bytes, request.index);
        const rgba = image.rgba();
        const pixels = { width: image.width(), height: image.height(), rgba };
        image.free();
        transfer.push(rgba.buffer);
        response = { id: request.id, ok: true, pixels };
        break;
      }
      case 'describe':
        response = { id: request.id, ok: true, value: icon_describe(request.bytes) };
        break;
      case 'extract': {
        const data = icon_extract_png(request.bytes, request.index);
        transfer.push(data.buffer);
        response = { id: request.id, ok: true, data };
        break;
      }
      case 'select': {
        const data = icon_select(request.bytes, Uint32Array.from(request.indices));
        transfer.push(data.buffer);
        response = { id: request.id, ok: true, data };
        break;
      }
      case 'faviconPack': {
        if (!source) throw new Error('No picture is open.');
        const made = source.favicon_pack(JSON.stringify(request.options), JSON.stringify(request.meta));
        const zipBytes = made.zip();
        const pack = { zip: zipBytes, snippet: made.snippet(), warnings: JSON.parse(made.warnings()) as EngineMessage[] };
        made.free();
        transfer.push(zipBytes.buffer);
        response = { id: request.id, ok: true, pack };
        break;
      }
      case 'faviconSnippet':
        response = { id: request.id, ok: true, value: favicon_snippet(request.hasSvg, request.themeColor) };
        break;
      case 'zipOpen': {
        zip?.free();
        zip = undefined;
        zip = ZipReader.open(request.bytes);
        response = { id: request.id, ok: true, value: zip.files() };
        break;
      }
      case 'zipRead': {
        if (!zip) throw new Error('No ZIP is open.');
        const data = zip.read(request.index);
        transfer.push(data.buffer);
        response = { id: request.id, ok: true, data };
        break;
      }
      case 'zipBuild': {
        const builder = new ZipBuilder();
        try {
          for (const file of request.files) builder.add(file.name, file.bytes);
          const data = builder.finish();
          transfer.push(data.buffer);
          response = { id: request.id, ok: true, data };
        } finally {
          builder.free();
        }
        break;
      }
      case 'pngZip': {
        const data = icon_png_zip(request.bytes, request.stem);
        transfer.push(data.buffer);
        response = { id: request.id, ok: true, data };
        break;
      }
      case 'close':
        forget();
        forgetGif();
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
