import { mount } from 'svelte';
import '@fontsource-variable/source-sans-3/wght.css';
import './app.css';
import App from './App.svelte';
import { loadLocale } from './i18n';
import { startLanguageSwitch } from './language';
import './menus';
import { startThemeSwitch } from './theme';

// The page is in one language, named in <html lang> (see seo.ts); its texts are loaded before anything is drawn.
await loadLocale(document.documentElement.lang);
startThemeSwitch();
startLanguageSwitch();
mount(App, { target: document.getElementById('app')! });
