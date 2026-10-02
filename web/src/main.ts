import { mount } from 'svelte';
import './app.css';
import App from './App.svelte';
import { locale, setLocale } from './i18n';

setLocale(navigator.language);
document.documentElement.lang = locale();

// The menus in the top bar are plain <details>; clicking elsewhere or pressing
// Escape closes them.
document.addEventListener('click', (event) => {
  document.querySelectorAll<HTMLDetailsElement>('details.menu[open]').forEach((menu) => {
    if (!menu.contains(event.target as Node)) menu.open = false;
  });
});
document.addEventListener('keydown', (event) => {
  if (event.key !== 'Escape') return;
  document.querySelectorAll<HTMLDetailsElement>('details.menu[open]').forEach((menu) => {
    menu.open = false;
    menu.querySelector('summary')?.focus();
  });
});

mount(App, { target: document.getElementById('app')! });
