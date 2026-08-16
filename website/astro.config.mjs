// @ts-check
import { defineConfig } from 'astro/config';
import react from '@astrojs/react';
import sitemap from '@astrojs/sitemap';
import tailwindcss from '@tailwindcss/vite';
import { unified } from '@astrojs/markdown-remark';
import rehypeCallouts from './src/plugins/rehype-callouts.mjs';
import rehypeMermaidClass from './src/plugins/rehype-mermaid-class.mjs';
import rehypeMermaid from 'rehype-mermaid';

// Static marketing site — two locales, path-prefixed (/en/, /id/), no
// server runtime. See website-plan.md §10 for the Cloudflare Pages settings.
// NOTE: live at a workers.dev URL until the oz-pos.com domain is bought;
// swap `site` to https://oz-pos.com (and robots.txt) when the custom
// domain goes live — canonical, og:url, sitemap, and hreflang all derive
// from this value.
export default defineConfig({
  site: 'https://oz-pos.adikaradwiatmaja.workers.dev',
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
      // or /login (form-only). The docs index is a 301 in _redirects, so it
      // has no indexable content either (the individual /docs/* pages stay).
      filter: (page) =>
        !/\/account\/$/.test(page) &&
        !/\/login\/$/.test(page) &&
        !/\/docs\/$/.test(page),
    }),
  ],
  markdown: {
    // Astro 7: remark/rehype plugins now live on the unified() processor
    // (top-level markdown.rehypePlugins is deprecated and prints a warning).
    // rehypeMermaid renders ```mermaid blocks to inline SVG at build time
    // (Playwright, browser at build only — zero client JS; see
    // src/content/docs/en/docs-authoring.md → Charts & diagrams).
    // rehypeMermaidClass must run first: Astro's shiki marks the block with
    // data-language="mermaid" and no class, which rehype-mermaid won't match.
    processor: unified({ rehypePlugins: [rehypeCallouts, rehypeMermaidClass, rehypeMermaid] }),
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
