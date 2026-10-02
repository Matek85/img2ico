// A look into an .ico file the engine made, just far enough to show each
// image of it. (Checking a file for problems is the engine's job, see
// validate_ico; this reads files the engine has just written.)

export interface IconEntry {
  /** Edge length in pixels. */
  size: number;
  /** The image data: a complete PNG. */
  png: Uint8Array;
}

const PNG_SIGNATURE = [0x89, 0x50, 0x4e, 0x47];

/** The PNG images in an .ico file, in the order of its directory. */
export function iconEntries(bytes: Uint8Array): IconEntry[] {
  if (bytes.length < 6 || bytes[2] !== 1 || bytes[3] !== 0) return [];
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const count = view.getUint16(4, true);
  const entries: IconEntry[] = [];
  for (let i = 0; i < count; i++) {
    const at = 6 + i * 16;
    if (at + 16 > bytes.length) break;
    const width = bytes[at] || 256;
    const length = view.getUint32(at + 8, true);
    const offset = view.getUint32(at + 12, true);
    const data = bytes.subarray(offset, offset + length);
    if (data.length === length && PNG_SIGNATURE.every((b, k) => data[k] === b)) {
      entries.push({ size: width, png: data });
    }
  }
  return entries;
}
