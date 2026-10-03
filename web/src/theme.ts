// Light or dark. The page follows the system's setting until the visitor switches it; the switch
// is remembered in this browser. The choice is the `data-theme` of the root element, which the
// colours in app.css answer to (an inline script in every page's head sets it before the page is
// drawn, see seo.ts, so there is no flash of the other theme).
import { t } from './i18n';

const KEY = 'img2ico.theme.v1';

export type Theme = 'light' | 'dark';

function stored(): Theme | null {
  try {
    const value = localStorage.getItem(KEY);
    return value === 'light' || value === 'dark' ? value : null;
  } catch {
    return null;
  }
}

function remember(theme: Theme | null): void {
  try {
    if (theme) localStorage.setItem(KEY, theme);
    else localStorage.removeItem(KEY);
  } catch {
    // Private window or blocked storage: the choice lasts until the page is closed.
  }
}

const system = window.matchMedia('(prefers-color-scheme: dark)');

/** The theme the page is shown in now. */
export function currentTheme(): Theme {
  const chosen = document.documentElement.dataset.theme;
  if (chosen === 'light' || chosen === 'dark') return chosen;
  return system.matches ? 'dark' : 'light';
}

function apply(choice: Theme | null): void {
  if (choice) document.documentElement.dataset.theme = choice;
  else delete document.documentElement.dataset.theme;
}

function showButton(button: HTMLButtonElement): void {
  const label = t(currentTheme() === 'dark' ? 'nav.theme_to_light' : 'nav.theme_to_dark');
  button.title = label;
  button.setAttribute('aria-label', label);
}

/**
 * Brings the switch of the top bar to life: it is disabled in the page's HTML (it cannot work without
 * a script). A click changes to the other theme; choosing what the system has anyway goes back to
 * following the system.
 */
export function startThemeSwitch(): void {
  apply(stored());
  const button = document.querySelector<HTMLButtonElement>('.theme-switch');
  if (!button) return;
  button.disabled = false;
  showButton(button);
  button.addEventListener('click', () => {
    const next: Theme = currentTheme() === 'dark' ? 'light' : 'dark';
    const systemTheme: Theme = system.matches ? 'dark' : 'light';
    const choice = next === systemTheme ? null : next;
    remember(choice);
    apply(choice);
    showButton(button);
  });
  // The system's setting changes (and nothing was chosen): the label follows.
  system.addEventListener('change', () => showButton(button));
}
