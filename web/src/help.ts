// The help pages: static text, the same top bar and style as the converter.
import './menus';
import { loadLocale } from './i18n';
import { startThemeSwitch } from './theme';

await loadLocale(document.documentElement.lang);
startThemeSwitch();
