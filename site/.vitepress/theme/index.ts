import DefaultTheme from 'vitepress/theme-without-fonts';
import type { Theme } from 'vitepress';
import ProductHome from './ProductHome.vue';
import '../../../web/src/design/tokens.css';
import './style.css';
import './home.css';

const reducedMotion = () => matchMedia('(prefers-reduced-motion: reduce)').matches;

// VitePress toggles the `dark` class; the shared tokens read data-theme.
function mirrorTheme() {
  const root = document.documentElement;
  const sync = () => root.setAttribute('data-theme', root.classList.contains('dark') ? 'dark' : 'light');
  sync();
  new MutationObserver(sync).observe(root, { attributes: true, attributeFilter: ['class'] });
}

// Pages fade and rise 4px on route change while the topbar and sidebar stay still.
function arrive() {
  if (reducedMotion()) return;
  requestAnimationFrame(() => {
    const content = document.querySelector('.VPContent .content-container, .VPContent .rc-product');
    content?.animate(
      [{ opacity: 0, transform: 'translateY(4px)' }, { opacity: 1, transform: 'none' }],
      { duration: 220, easing: 'cubic-bezier(0.16, 1, 0.3, 1)' },
    );
  });
}

export default {
  extends: DefaultTheme,
  enhanceApp({ app, router }) {
    app.component('ProductHome', ProductHome);
    if (typeof document === 'undefined') return;
    mirrorTheme();
    const previous = router.onAfterRouteChange;
    router.onAfterRouteChange = async to => {
      await previous?.(to);
      arrive();
    };
  },
} satisfies Theme;
