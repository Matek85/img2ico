// The queue itself: what was added, kept while the page is open. Nothing leaves
// the browser, and nothing is kept after it is closed.
import { outputName } from './batch';
import { histories } from './history';
import { iconEntries } from './ico';
import { moveItem } from './queue';
import type { Settings } from './settings';

type QueueFormat = 'ico' | 'icns';

export interface QueueItem {
  id: number;
  /** An icon made from a picture (it can be edited again), or an .ico file that was added as it is. */
  kind: 'picture' | 'icon';
  /** The picture the icon was made from (or the name of the .ico file), and the file itself. */
  picture: string;
  file: File;
  /** The settings it was made with; none for an .ico file added as it is. */
  settings: Settings | null;
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
// What the editor next to the queue still has to put into it (see flush).
let flusher: (() => Promise<void>) | null = null;
// The icon added last, for a moment: its row flashes.
let fresh = $state<number | null>(null);
let freshTimer: ReturnType<typeof setTimeout> | undefined;

function markFresh(id: number) {
  fresh = id;
  clearTimeout(freshTimer);
  freshTimer = setTimeout(() => (fresh = null), 1800);
}

/** `list` with `item` put right after the item `afterId` (at the end if there is none). */
function inserted(list: QueueItem[], item: QueueItem, afterId?: number): QueueItem[] {
  const at = afterId === undefined ? -1 : list.findIndex((other) => other.id === afterId);
  return at < 0 ? [...list, item] : [...list.slice(0, at + 1), item, ...list.slice(at + 1)];
}

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

  get ticks(): number {
    return ticks;
  },

  get problems(): string[] {
    return problems;
  },

  get fresh(): number | null {
    return fresh;
  },

  setProblems(list: string[]) {
    problems = list;
  },

  /** The editor says how to bring the queue up to date with what is being edited. */
  setFlusher(job: (() => Promise<void>) | null) {
    flusher = job;
  },

  /** Brings the queue up to date; what reads it (a download) calls this first, so no last change is missed. */
  async flush(): Promise<void> {
    await flusher?.();
  },

  find(id: number): QueueItem | undefined {
    return items.find((item) => item.id === id);
  },

  add(picture: string, made: Made): QueueItem {
    const made_ = parts(made);
    const item: QueueItem = {
      id: nextId++,
      kind: 'picture',
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
    markFresh(item.id);
    return item;
  },

  /** Adds an .ico file as it is: it is not made from a picture, so it cannot be edited, only looked into. */
  addIcon(
    name: string,
    file: File,
    bytes: Uint8Array,
    sizes: number[],
    largestPng: Uint8Array | null,
    afterId?: number,
  ): QueueItem {
    const item: QueueItem = {
      id: nextId++,
      kind: 'icon',
      picture: name,
      file,
      settings: null,
      fileName: outputName(
        items.map((existing) => existing.fileName),
        name,
        'ico',
      ),
      format: 'ico',
      bytes,
      sizes,
      thumb: largestPng ? URL.createObjectURL(new Blob([largestPng as BlobPart], { type: 'image/png' })) : '',
    };
    items = inserted(items, item, afterId);
    notice = true;
    ticks += 1;
    markFresh(item.id);
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
    histories.drop(id);
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
    histories.clear();
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
