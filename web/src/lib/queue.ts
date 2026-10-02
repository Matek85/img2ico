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

/** Whether the icons can be combined into one .ico file: two or more, all of them .ico. */
export function canCombine(formats: readonly string[]): boolean {
  return formats.length >= 2 && formats.every((format) => format === 'ico');
}

/** The sizes of the images, as text: "16, 32, 48". */
export function sizesText(sizes: readonly number[]): string {
  return sizes.join(', ');
}
