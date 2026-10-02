// Handing a file to the person: the browser's own "save as" for bytes that
// only exist in memory.

/** Saves `bytes` as a file called `name`. */
export function saveBytes(bytes: Uint8Array, name: string, type: string): void {
  const url = URL.createObjectURL(new Blob([bytes as BlobPart], { type }));
  const link = document.createElement('a');
  link.href = url;
  link.download = name;
  link.click();
  // Give the browser time to start the download before the address goes away.
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

export const ICO_TYPE = 'image/x-icon';
export const ICNS_TYPE = 'image/icns';
export const PNG_TYPE = 'image/png';
export const ZIP_TYPE = 'application/zip';
