/** One image inside an .ico file, as the engine describes it. */
export interface IconImage {
  index: number;
  width: number;
  height: number;
  bits_per_pixel: number;
  format: string;
  offset: number;
  size: number;
}

/** A problem the check found; `image` is the zero-based entry it is about. */
export interface Finding {
  image: number | null;
  message: string;
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
