// The queue itself: what was added, kept while the page is open. Nothing leaves
// the browser, and nothing is kept after it is closed.
import { outputName } from './batch';
import { iconEntries } from './ico';
import { canCombine, moveItem } from './queue';

export type QueueFormat = 'ico' | 'icns';

export interface QueueItem {
  id: number;
  /** The picture the icon was made from. */
  picture: string;
  /** The name of the icon file, unique in the queue. */
  fileName: string;
  format: QueueFormat;
  bytes: Uint8Array;
  /** The sizes of an .ico (empty for an .icns, which has its own set). */
  sizes: number[];
  /** The largest image, as an address for <img>. */
  thumb: string;
}

let items = $state<QueueItem[]>([]);
let nextId = 1;
// Set when something was added, until the queue is shown (see takeNotice).
let notice = false;

function release(item: QueueItem) {
  if (item.thumb) URL.revokeObjectURL(item.thumb);
}

export const queue = {
  get items(): QueueItem[] {
    return items;
  },

  get canCombine(): boolean {
    return canCombine(items.map((item) => item.format));
  },

  /**
   * Adds an icon. `preview` is an .ico with the images to show (for an .icns
   * the preview conversion, since the page does not read .icns files).
   */
  add(picture: string, format: QueueFormat, bytes: Uint8Array, preview: Uint8Array): QueueItem {
    const entries = iconEntries(preview);
    const largest = entries[entries.length - 1];
    const item: QueueItem = {
      id: nextId++,
      picture,
      fileName: outputName(
        items.map((existing) => existing.fileName),
        picture,
        format,
      ),
      format,
      bytes,
      sizes: format === 'ico' ? entries.map((entry) => entry.size) : [],
      thumb: largest ? URL.createObjectURL(new Blob([largest.png as BlobPart], { type: 'image/png' })) : '',
    };
    items = [...items, item];
    notice = true;
    return item;
  },

  remove(id: number) {
    const gone = items.find((item) => item.id === id);
    if (gone) release(gone);
    items = items.filter((item) => item.id !== id);
  },

  move(id: number, delta: number) {
    items = moveItem(
      items,
      items.findIndex((item) => item.id === id),
      delta,
    );
  },

  clear() {
    items.forEach(release);
    items = [];
  },

  /** True once after something was added: the page then shows where it went. */
  takeNotice(): boolean {
    const was = notice;
    notice = false;
    return was;
  },
};
