// The queue itself: what was added, kept while the page is open. Nothing leaves
// the browser, and nothing is kept after it is closed.
import { outputName } from './batch';
import { iconEntries } from './ico';
import { canCombine, moveItem } from './queue';
import type { Settings } from './settings';

export type QueueFormat = 'ico' | 'icns';

export interface QueueItem {
  id: number;
  /** The picture the icon was made from, and the picture itself, so the icon can be edited again. */
  picture: string;
  file: File;
  /** The settings it was made with. */
  settings: Settings;
  /** The name of the icon file, unique in the queue. */
  fileName: string;
  format: QueueFormat;
  bytes: Uint8Array;
  /** The sizes of an .ico (empty for an .icns, which has its own set). */
  sizes: number[];
  /** The largest image, as an address for <img>. */
  thumb: string;
}

/** What an icon is made of: the result of working on a picture. */
export interface Made {
  file: File;
  settings: Settings;
  format: QueueFormat;
  bytes: Uint8Array;
  /** An .ico with the images to show (for an .icns the preview conversion, since the page does not read .icns files). */
  preview: Uint8Array;
}

let items = $state<QueueItem[]>([]);
let nextId = 1;
// Set when something was added or changed, until the queue is shown (see takeNotice).
let notice = false;
// Counts the additions and changes, so a queue that is already shown can flash.
let ticks = $state(0);
// What could not be added, or was left out, when pictures were added in bulk.
let problems = $state<string[]>([]);

function release(item: QueueItem) {
  if (item.thumb) URL.revokeObjectURL(item.thumb);
}

/** The parts of an item that come from `made`. */
function parts(made: Made) {
  const entries = iconEntries(made.preview);
  const largest = entries[entries.length - 1];
  return {
    file: made.file,
    settings: $state.snapshot(made.settings) as Settings,
    format: made.format,
    bytes: made.bytes,
    sizes: made.format === 'ico' ? entries.map((entry) => entry.size) : [],
    thumb: largest ? URL.createObjectURL(new Blob([largest.png as BlobPart], { type: 'image/png' })) : '',
  };
}

export const queue = {
  get items(): QueueItem[] {
    return items;
  },

  get canCombine(): boolean {
    return canCombine(items.map((item) => item.format));
  },

  get ticks(): number {
    return ticks;
  },

  get problems(): string[] {
    return problems;
  },

  setProblems(list: string[]) {
    problems = list;
  },

  find(id: number): QueueItem | undefined {
    return items.find((item) => item.id === id);
  },

  add(picture: string, made: Made): QueueItem {
    const made_ = parts(made);
    const item: QueueItem = {
      id: nextId++,
      picture,
      fileName: outputName(
        items.map((existing) => existing.fileName),
        picture,
        made.format,
      ),
      ...made_,
    };
    items = [...items, item];
    notice = true;
    ticks += 1;
    return item;
  },

  /** Replaces what an icon is made of, keeping its place; `announce` lets the queue show the change when it is next shown. */
  update(id: number, made: Made, announce = true) {
    const old = items.find((item) => item.id === id);
    if (!old) return;
    release(old);
    const others = items.filter((item) => item.id !== id).map((item) => item.fileName);
    const fileName = made.format === old.format ? old.fileName : outputName(others, old.picture, made.format);
    items = items.map((item) => (item.id === id ? { ...item, ...parts(made), fileName } : item));
    if (announce) {
      notice = true;
      ticks += 1;
    }
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
    problems = [];
  },

  /** True once after something was added or changed: the page then shows where it went. */
  takeNotice(): boolean {
    const was = notice;
    notice = false;
    return was;
  },
};
