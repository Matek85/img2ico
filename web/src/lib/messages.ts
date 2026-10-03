// What the engine says - its warnings and errors - as sentences for this page.
//
// The engine (shared with the command line) hands over a code, the values in the message and
// its English sentence (see crates/core/src/msg.rs). The sentence for the page comes from the
// catalogue under `msg.<code>`, so it is written in the visitor's language; a value that is a
// message itself (a reason inside "could not read ...") is turned into text first. A message
// that has no code of its own is shown as the English sentence it came with, with the names of
// command line options put into words.

import { t } from '../i18n';
import type { EngineMessage } from '../engine/protocol';

/** An error from the engine, with the message it was made of when it had one. */
export class EngineError extends Error {
  readonly info?: EngineMessage;

  constructor(raw: string) {
    let info: EngineMessage | undefined;
    if (raw.startsWith('{"code"')) {
      try {
        info = JSON.parse(raw) as EngineMessage;
      } catch {
        info = undefined;
      }
    }
    super(info ? info.text : raw);
    this.name = 'EngineError';
    this.info = info;
  }
}

// Option names, in words, for a sentence without a code.
const OPTIONS: Record<string, string> = {
  tolerance: 'msg.option_tolerance',
  'chroma-key': 'msg.option_background',
  find: 'msg.option_background',
  trim: 'msg.option_trim',
  padding: 'msg.option_padding',
  crop: 'msg.option_crop',
  seed: 'msg.option_point',
};

function plain(text: string): string {
  const clean = text.replace(/^Warning:\s*/, '').trim();
  const words = clean.replace(/--([a-z][a-z-]*)/g, (_, name: string) => (OPTIONS[name] ? t(OPTIONS[name]) : name.replace(/-/g, ' ')));
  return words.charAt(0).toUpperCase() + words.slice(1);
}

/** A message of the engine as a sentence for this page. */
export function describeMessage(message: EngineMessage): string {
  if (message.code === 'other') return plain(message.text);
  const params: Record<string, string> = {};
  for (const [name, value] of Object.entries(message.params ?? {})) {
    params[name] = typeof value === 'string' ? value : describeMessage(value);
  }
  const key = `msg.${message.code}`;
  const sentence = t(key, params);
  // A code the catalogue does not know yet: the English sentence still says what happened.
  return sentence === key ? plain(message.text) : sentence;
}

/** An English sentence of the engine without a code (an older caller, a message made in the page) as text for this page. */
export function friendly(text: string): string {
  return plain(text);
}

/** The text of whatever was thrown, as a sentence for this page. */
export function explain(error: unknown): string {
  if (error instanceof EngineError && error.info) return describeMessage(error.info);
  return plain(error instanceof Error ? error.message : String(error));
}
