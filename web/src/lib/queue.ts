// The queue of finished icons: pictures turned into icons one after the other,
// to be downloaded together at the end. This is the part without state.

/** A list with the item at `index` moved by `delta` places (up is negative); the same list if it cannot move. */
export function moveItem<T>(list: readonly T[], index: number, delta: number): T[] {
  const target = index + delta;
  if (index < 0 || index >= list.length || target < 0 || target >= list.length || delta === 0) {
    return [...list];
  }
  const copy = [...list];
  const [item] = copy.splice(index, 1);
  copy.splice(target, 0, item);
  return copy;
}

/**
 * Where a row dragged to the height `y` would be dropped: the number of rows whose middle is above `y`
 * (0 is before the first row, `middles.length` after the last).
 */
export function dropSlot(middles: readonly number[], y: number): number {
  return middles.filter((middle) => middle < y).length;
}

/** The place of the dragged row in the list once it is dropped in `slot` (a slot counts the row itself). */
export function placeAfterDrop(from: number, slot: number): number {
  return slot > from ? slot - 1 : slot;
}

/** The sizes of the images, as text: "16, 32, 48". */
export function sizesText(sizes: readonly number[]): string {
  return sizes.join(', ');
}
