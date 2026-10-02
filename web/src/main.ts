import { mount } from 'svelte';
import './app.css';
import App from './App.svelte';
import { locale, setLocale } from './i18n';

setLocale(navigator.language);
document.documentElement.lang = locale();

mount(App, { target: document.getElementById('app')! });
