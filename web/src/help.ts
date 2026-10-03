// The help pages: static text, the same top bar and style as the converter.
import '@fontsource-variable/source-sans-3/wght.css';
import './app.css';
import './menus';
import { loadLocale } from './i18n';
import { startThemeSwitch } from './theme';

await loadLocale(document.documentElement.lang);
startThemeSwitch();
