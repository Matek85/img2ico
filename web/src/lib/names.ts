// File names can be of any length; where one is shown inside a sentence or on a
// button it is shortened in the middle, keeping the start and the file type.

/** `name` as it is when it fits in `max` characters, else its start, "…" and its file type. */
export function shortName(name: string, max = 32): string {
  if (name.length <= max) return name;
  const extension = /\.[A-Za-z0-9]{1,5}$/.exec(name)?.[0] ?? '';
  const keep = Math.max(1, max - 1 - extension.length);
  return `${name.slice(0, keep)}…${extension}`;
}
