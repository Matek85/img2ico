// All texts of the page. Nothing in a component is written in a language
// directly: components ask for a key, so another language is one more file of
// this shape (see index.ts). Plural forms are keys with the suffix _one /
// _other (the suffixes of Intl.PluralRules); {name} is replaced by a value.
export const en = {
  'app.name': 'img2ico',
  'app.tagline': 'Check your icon files in the browser.',
  'app.privacy': 'Your files never leave your browser. Nothing is uploaded.',

  'drop.prompt': 'Drop an .ico file here',
  'drop.or': 'or',
  'drop.choose': 'Choose a file',
  'drop.hint': 'The file is checked on this device.',
  'drop.active': 'Release to check the file',

  'state.working': 'Checking {name} …',
  'state.failed': 'Could not check the file: {reason}',

  'result.valid': 'This icon file is valid.',
  'result.invalid': 'This icon file has problems.',
  'result.file': '{name}, {size}',
  'result.images_one': '{count} image',
  'result.images_other': '{count} images',
  'result.errors': 'Errors',
  'result.warnings': 'Warnings',
  'result.about_image': 'Image {number}: {message}',
  'result.another': 'Check another file',
  'image.format': '{width} × {height}, {format}, {bits}-bit',

  'footer.engine': 'Engine {version}',
} as const;

export type MessageKey = keyof typeof en;
