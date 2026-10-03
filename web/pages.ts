// Every page exists once as a template (index.html, help/<topic>/index.html). A page of another language is the
// same template in the folder of that language (de/index.html, de/help/<topic>/index.html, ...): Vite builds a page
// per file, and the plugin in seo.ts makes each of them speak its language. The copies are made here, whenever the
// build or the dev server starts, and are not kept in git.
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { HELP_TOPICS, LANGUAGES } from './seo.ts';

/** The templates, below the root of the site. */
export const TEMPLATES = ['index.html', ...HELP_TOPICS.map((topic) => `help/${topic.slug}/index.html`)];

/** The pages of the languages other than English, below the root of the site. */
export function languagePages(): string[] {
  return LANGUAGES.filter((language) => language.code !== 'en').flatMap((language) =>
    TEMPLATES.map((template) => `${language.code}/${template}`),
  );
}

/** Makes the copies below `root` (the folder of the templates). */
export function writeLanguagePages(root: URL): void {
  for (const page of languagePages()) {
    const template = page.slice(page.indexOf('/') + 1);
    const target = new URL(page, root);
    mkdirSync(new URL('.', target), { recursive: true });
    writeFileSync(target, readFileSync(new URL(template, root), 'utf8'));
  }
}
