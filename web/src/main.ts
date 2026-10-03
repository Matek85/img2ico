import { mount } from 'svelte';
import App from './App.svelte';
import { loadLocale } from './i18n';
import { startLanguageSwitch } from './language';
import { keyName, readLayout } from './lib/layout.svelte';
import { useKeyNames } from './lib/shortcuts';
import './menus';
import { startThemeSwitch } from './theme';

// The page is in one language, named in <html lang> (see seo.ts); its texts are loaded before anything is drawn.
await loadLocale(document.documentElement.lang);
startThemeSwitch();
startLanguageSwitch();
readLayout();
useKeyNames(keyName);
mount(App, { target: document.getElementById('app')! });
