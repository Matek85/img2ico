// Several pictures at once: which dropped files count as pictures, what the
// icons made from them are called, and the shape of one item of the list.

/** One picture of a batch; its bytes are read when they are needed. */
export interface BatchItem {
  /** The name shown, and the name the icon is made from: a file name, or a path inside a ZIP. */
  name: string;
  load: () => Promise<Uint8Array>;
}

/** The kinds of picture the engine reads. */
const PICTURE_EXTENSIONS = ['png', 'jpg', 'jpeg', 'gif', 'webp', 'bmp', 'tif', 'tiff', 'tga', 'svg', 'icns'];

function extensionOf(name: string): string {
  const dot = name.lastIndexOf('.');
  return dot < 0 ? '' : name.slice(dot + 1).toLowerCase();
}

/** The last part of a path: "art/logo.png" is "logo.png". */
export function baseName(path: string): string {
  return path.slice(path.lastIndexOf('/') + 1);
}

/** Whether the name looks like a picture the engine can read. */
export function isPictureName(name: string): boolean {
  return PICTURE_EXTENSIONS.includes(extensionOf(name));
}

export function isIconName(name: string): boolean {
  return ['ico', 'cur'].includes(extensionOf(name));
}

export function isZipName(name: string): boolean {
  return extensionOf(name) === 'zip';
}

/** A name without its extension. */
export function stemOf(name: string): string {
  const base = baseName(name);
  const dot = base.lastIndexOf('.');
  return (dot > 0 ? base.slice(0, dot) : base) || 'icon';
}

/**
 * The name of the icon made from `picture`: its name with the icon's
 * extension, or `..._2.ico` and so on when `taken` already has that name (two
 * pictures of the same name from different folders of a ZIP).
 */
export function outputName(taken: readonly string[], picture: string, extension: string): string {
  const stem = stemOf(picture);
  let name = `${stem}.${extension}`;
  for (let n = 2; taken.includes(name); n++) {
    name = `${stem}_${n}.${extension}`;
  }
  return name;
}

/** The most pictures one batch takes: more would be hard to look through and slow to wait for. */
export const MAX_BATCH = 500;
