import DefaultTheme from 'vitepress/theme-without-fonts';
import type { Theme } from 'vitepress';
import ProductHome from './ProductHome.vue';
import './style.css';

export default {
  extends: DefaultTheme,
  enhanceApp({ app }) { app.component('ProductHome', ProductHome); },
} satisfies Theme;
