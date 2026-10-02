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

/** What combining does to one icon: which of its sizes are already there from icons above it. */
export interface Overlap {
  name: string;
  /** The sizes left out, and how many sizes the icon has. */
  left: number[];
  total: number;
}

/**
 * An .ico file holds one image per size, so when icons are combined the first
 * image of each size wins. This tells, for each icon from the second on, which
 * of its sizes would be left out; icons that lose nothing are not listed.
 */
export function overlaps(icons: readonly { name: string; sizes: readonly number[] }[]): Overlap[] {
  const seen = new Set<number>();
  const found: Overlap[] = [];
  icons.forEach((icon, at) => {
    const unique = [...new Set(icon.sizes)];
    const left = unique.filter((size) => seen.has(size));
    if (at > 0 && left.length > 0) found.push({ name: icon.name, left, total: unique.length });
    unique.forEach((size) => seen.add(size));
  });
  return found;
}
