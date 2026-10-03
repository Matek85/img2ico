// The menus in the top bar are plain <details>; clicking elsewhere or pressing
// Escape closes them.
document.addEventListener('click', (event) => {
  document.querySelectorAll<HTMLDetailsElement>('details.menu[open]').forEach((menu) => {
    if (!menu.contains(event.target as Node)) menu.open = false;
  });
});
// The language chosen in the menu is remembered, so the next visit goes to it (see languageRedirect in seo.ts).
document.addEventListener('click', (event) => {
  const link = (event.target as Element).closest<HTMLAnchorElement>('.menu.lang a[data-lang]');
  if (!link?.dataset.lang) return;
  try {
    localStorage.setItem('img2ico.lang.v1', link.dataset.lang);
  } catch {
    // Without storage the choice only counts for this click.
  }
});
document.addEventListener('keydown', (event) => {
  if (event.key !== 'Escape') return;
  document.querySelectorAll<HTMLDetailsElement>('details.menu[open]').forEach((menu) => {
    menu.open = false;
    menu.querySelector('summary')?.focus();
  });
});
