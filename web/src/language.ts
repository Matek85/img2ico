// Switching the language in the menu without leaving the page: the work in the editor and the queue stays.
// The other language's page is fetched, the parts around the app (top bar, heading, note, text below, footer
// line, head tags) are swapped in, and the app's own texts change because `t()` is reactive. If anything fails,
// the link is followed as usual.
import { loadLocale } from './i18n';
import { language } from './i18n/version.svelte';
import { startThemeSwitch } from './theme';

const HEAD_PARTS =
  'meta[name="description"], meta[property^="og:"], meta[name^="twitter:"], link[rel="canonical"], link[rel="alternate"], script[type="application/ld+json"]';

function swap(from: Document, selector: string, html?: (element: Element) => string): void {
  const next = from.querySelector(selector);
  const current = document.querySelector(selector);
  if (!next || !current) throw new Error(`missing ${selector}`);
  current.innerHTML = html ? html(next) : next.innerHTML;
}

async function switchTo(link: HTMLAnchorElement): Promise<void> {
  const code = link.dataset.lang!;
  const response = await fetch(link.href);
  if (!response.ok) throw new Error(String(response.status));
  const page = new DOMParser().parseFromString(await response.text(), 'text/html');
  await loadLocale(page.documentElement.lang);

  document.documentElement.lang = page.documentElement.lang;
  document.title = page.title;
  document.head.querySelectorAll(HEAD_PARTS).forEach((element) => element.remove());
  page.head.querySelectorAll(HEAD_PARTS).forEach((element) => document.head.append(document.importNode(element, true)));

  swap(page, 'header.topbar');
  swap(page, 'section.hero');
  swap(page, 'section.about');
  // The footer's engine line is filled by the app and stays; the first paragraph is the page's own text.
  const privacy = document.querySelector('footer p');
  const nextPrivacy = page.querySelector('footer p');
  if (privacy && nextPrivacy) privacy.innerHTML = nextPrivacy.innerHTML;

  document.querySelector('body > .ai-note')?.remove();
  const note = page.querySelector('body > .ai-note');
  if (note) document.getElementById('app')!.before(document.importNode(note, true));

  startThemeSwitch();
  language.version += 1;
  history.pushState(null, '', link.href);
  try {
    localStorage.setItem('img2ico.lang.v1', code);
  } catch {
    // Without storage the choice only counts for this visit.
  }
  document.querySelectorAll<HTMLDetailsElement>('details.menu[open]').forEach((menu) => (menu.open = false));
}

/** Makes the language links of the app page switch in place. Other pages (help) keep their ordinary links. */
export function startLanguageSwitch(): void {
  if (!document.getElementById('app')) return;
  document.addEventListener('click', (event) => {
    if (event.defaultPrevented || event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
    const link = (event.target as Element).closest<HTMLAnchorElement>('.menu.lang a[data-lang]');
    if (!link) return;
    event.preventDefault();
    switchTo(link).catch(() => {
      location.href = link.href;
    });
  });
  // The address changed by a language switch: the back button needs a page that fits it.
  window.addEventListener('popstate', () => location.reload());
}
