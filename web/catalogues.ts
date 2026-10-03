// The texts of every language as the build uses them: the page's own texts and the help pages' texts
// together. (The page in the browser loads only its own language, see src/i18n/index.ts, and has no use for
// the help texts, which are already in the static HTML.)
import { de } from './src/i18n/de.ts';
import { en } from './src/i18n/en.ts';
import { es } from './src/i18n/es.ts';
import { fr } from './src/i18n/fr.ts';
import { helpDe } from './src/i18n/help-de.ts';
import { helpEn } from './src/i18n/help-en.ts';
import { helpEs } from './src/i18n/help-es.ts';
import { helpFr } from './src/i18n/help-fr.ts';
import { helpPtBr } from './src/i18n/help-pt-br.ts';
import { ptBr } from './src/i18n/pt-br.ts';

export type Messages = Record<string, string>;

/** By language code. */
export const CATALOGUES: Record<string, Messages> = {
  en: { ...en, ...helpEn },
  de: { ...de, ...helpDe },
  es: { ...es, ...helpEs },
  'pt-br': { ...ptBr, ...helpPtBr },
  fr: { ...fr, ...helpFr },
};
