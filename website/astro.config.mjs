// @ts-check
import { defineConfig } from 'astro/config';
import react from '@astrojs/react';
import sitemap from '@astrojs/sitemap';
import tailwindcss from '@tailwindcss/vite';
import rehypeCallouts from './src/plugins/rehype-callouts.mjs';

// Static marketing site — two locales, path-prefixed (/en/, /id/), no
// server runtime. See website-plan.md §10 for the Cloudflare Pages settings.
export default defineConfig({
  site: 'https://oz-pos.com',
  integrations: [react(), sitemap()],
  markdown: {
    rehypePlugins: [rehypeCallouts],
  },
  vite: {
    plugins: [tailwindcss()],
  },
  i18n: {
    defaultLocale: 'en',
    locales: ['en', 'id'],
    routing: {
      // Both locales are path-prefixed: /en/… and /id/…
      prefixDefaultLocale: true,
    },
  },
});
