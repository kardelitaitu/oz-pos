// @ts-check
import { defineConfig } from 'astro/config';
import react from '@astrojs/react';
import sitemap from '@astrojs/sitemap';
import tailwindcss from '@tailwindcss/vite';
import { unified } from '@astrojs/markdown-remark';
import rehypeCallouts from './src/plugins/rehype-callouts.mjs';

// Static marketing site — two locales, path-prefixed (/en/, /id/), no
// server runtime. See website-plan.md §10 for the Cloudflare Pages settings.
export default defineConfig({
  site: 'https://oz-pos.com',
  integrations: [
    react(),
    sitemap({
      // Emit <xhtml:link rel="alternate" hreflang> pairs for both locales
      // so search engines treat /en/… and /id/… as translations of each other.
      i18n: {
        defaultLocale: 'en',
        locales: {
          en: 'en',
          id: 'id',
        },
      },
      // Skip the auth pages — no indexable content on /account (session-gated)
      // or /login (form-only).
      filter: (page) => !/\/account\/$/.test(page) && !/\/login\/$/.test(page),
    }),
  ],
  markdown: {
    // Astro 7: remark/rehype plugins now live on the unified() processor
    // (top-level markdown.rehypePlugins is deprecated and prints a warning).
    processor: unified({ rehypePlugins: [rehypeCallouts] }),
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
