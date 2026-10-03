import { mount } from 'svelte';
import '@fontsource-variable/source-sans-3/wght.css';
import './app.css';
import App from './App.svelte';
import { locale, setLocale } from './i18n';

setLocale(navigator.language);
document.documentElement.lang = locale();

import './menus';
import { startThemeSwitch } from './theme';

startThemeSwitch();
mount(App, { target: document.getElementById('app')! });
