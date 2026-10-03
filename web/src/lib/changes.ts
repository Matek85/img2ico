// What changed between two states of the settings, in the groups the person knows them by (the list of changes
// under the editor names these).

import type { Settings } from './settings';

export type ChangeGroup =
  | 'sizes'
  | 'padding'
  | 'corners'
  | 'fit'
  | 'grayscale'
  | 'trim'
  | 'flip'
  | 'rotate'
  | 'crop'
  | 'background'
  | 'format'
  | 'site';

const GROUP_OF: Record<Exclude<keyof Settings, 'gifFrame'>, ChangeGroup> = {
  sizes: 'sizes',
  padding: 'padding',
  cornerRadius: 'corners',
  fit: 'fit',
  grayscale: 'grayscale',
  trim: 'trim',
  flipH: 'flip',
  flipV: 'flip',
  rotate: 'rotate',
  crop: 'crop',
  removeBackground: 'background',
  backgroundAuto: 'background',
  backgroundColor: 'background',
  tolerance: 'background',
  feather: 'background',
  format: 'format',
  siteName: 'site',
  themeColor: 'site',
  appleBackground: 'site',
};

/** The groups of settings that differ between `before` and `after` (the shown frame of a GIF is not a setting). */
export function changedGroups(before: Settings, after: Settings): ChangeGroup[] {
  const groups = new Set<ChangeGroup>();
  for (const key of Object.keys(GROUP_OF) as (keyof typeof GROUP_OF)[]) {
    if (JSON.stringify(before[key]) !== JSON.stringify(after[key])) groups.add(GROUP_OF[key]);
  }
  return [...groups];
}
