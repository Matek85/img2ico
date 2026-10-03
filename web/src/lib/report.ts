import type { EngineMessage } from '../engine/protocol';

/** One image inside an .ico file, as the engine describes it. */
interface IconImage {
  index: number;
  width: number;
  height: number;
  bits_per_pixel: number;
  format: string;
  offset: number;
  size: number;
}

/** A problem the check found; `image` is the zero-based entry it is about. */
interface Finding {
  image: number | null;
  /** The engine's English sentence. */
  message: string;
  /** The same as a code with its values, for the page's own sentence. */
  info?: EngineMessage;
}

/** The engine's verdict on an .ico file (the shape of `img2ico --validate --json`). */
export interface ValidationReport {
  valid: boolean;
  bytes: number;
  images: IconImage[];
  errors: Finding[];
  warnings: Finding[];
}

export function parseReport(json: string): ValidationReport {
  const value = JSON.parse(json) as ValidationReport;
  if (typeof value.valid !== 'boolean' || !Array.isArray(value.images)) {
    throw new Error('The engine returned an unexpected result.');
  }
  return value;
}
