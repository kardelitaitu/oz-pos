#!/usr/bin/env node
/**
 * scripts/inject-module-preloads.mjs — flatten the island JS waterfall.
 *
 * Astro islands hydrate through dynamic imports: the browser only discovers
 * HeroCarousel.js after parsing the HTML, then client.js only after
 * downloading HeroCarousel.js, then react/react-dom/jsx-runtime only after
 * downloading client.js — a 4-deep discovery chain that shows up in
 * Lighthouse's "Network dependency tree" (~570ms max critical-path latency).
 * Astro emits no <link rel="modulepreload"> for island chunks.
 *
 * This postbuild step walks each built HTML page's island entry points
 * (astro-island component-url / renderer-url + <script type="module" src>),
 * resolves their STATIC import graph from the real chunks in dist/_astro/,
 * and injects a deduped block of <link rel="modulepreload"> tags into <head>.
 * The browser then fetches every chunk in parallel, one hop from the HTML;
 * the later dynamic import() hits the in-flight/loaded module instantly.
 *
 * Idempotent: a previous injection block (marker comments) is stripped
 * before re-injecting, so re-running on a dirty dist is always correct.
 *
 * Usage: node scripts/inject-module-preloads.mjs [distDir]
 */
import { readdirSync, readFileSync, writeFileSync, existsSync } from 'node:fs';
import { join } from 'node:path';

const DIST = process.argv[2] || join(process.cwd(), 'dist');
const OPEN = '<!-- injected by scripts/inject-module-preloads.mjs (modulepreload for island graphs) -->';
const CLOSE = '<!-- end modulepreload injection -->';

/** Recursively collect .html files under dir. */
function htmlFiles(dir) {
  const out = [];
  for (const name of readdirSync(dir, { withFileTypes: true })) {
    const p = join(dir, name.name);
    if (name.isDirectory()) out.push(...htmlFiles(p));
    else if (name.name.endsWith('.html')) out.push(p);
  }
  return out;
}

/** Resolve a relative import specifier from an /_astro/ chunk to a site URL. */
function resolveSpec(spec, fromUrl) {
  if (!spec) return null;
  try {
    const u = new URL(spec, `https://site${fromUrl}`);
    if (u.pathname.startsWith('/_astro/') && u.pathname.endsWith('.js')) return u.pathname;
  } catch { /* malformed specifier — ignore */ }
  return null;
}

/** Extract STATIC import specifiers from a chunk's source (dynamic import() is left lazy). */
function staticImports(src) {
  const specs = [];
  const re = /(?:from|import)\s*["'](\.[^"']+)["']/g;
  let m;
  while ((m = re.exec(src))) specs.push(m[1]);
  return specs;
}

/** BFS the static import graph of one chunk URL → transitive /_astro/*.js URLs. */
function graphOf(entryUrl) {
  const seen = new Set();
  const queue = [entryUrl];
  while (queue.length) {
    const url = queue.pop();
    if (seen.has(url)) continue;
    seen.add(url);
    const file = join(DIST, url.replace(/^\//, ''));
    if (!existsSync(file)) continue;
    for (const spec of staticImports(readFileSync(file, 'utf8'))) {
      const next = resolveSpec(spec, url);
      if (next && !seen.has(next)) queue.push(next);
    }
  }
  return seen;
}

/** URLs this page needs as one-hop module preloads. */
function pagePreloads(html) {
  const entries = new Set();
  for (const m of html.matchAll(/(?:component-url|renderer-url)="([^"]+)"/g)) {
    if (m[1].startsWith('/_astro/') && m[1].endsWith('.js')) entries.add(m[1]);
  }
  for (const tag of html.matchAll(/<script\b[^>]*>/g)) {
    const src = tag[0].match(/\ssrc="(\/_astro\/[^"]+\.js)"/);
    if (src && /\btype=["']module["']/.test(tag[0])) entries.add(src[1]);
  }
  const preloads = new Set();
  for (const url of entries) for (const u of graphOf(url)) preloads.add(u);
  return [...preloads].sort();
}

let touched = 0;
for (const file of htmlFiles(DIST)) {
  const original = readFileSync(file, 'utf8');
  // Strip a previous injection block, then collect entry URLs from the clean HTML.
  const clean = original.replace(
    new RegExp(`${OPEN.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}[\\s\\S]*?${CLOSE.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}\n?`),
    '',
  );
  const urls = pagePreloads(clean);
  if (!urls.length) continue;
  if (clean.includes('</head>')) {
    const block = `${OPEN}\n${urls.map((u) => `<link rel="modulepreload" crossorigin href="${u}" />`).join('\n')}\n${CLOSE}\n`;
    writeFileSync(file, clean.replace('</head>', `${block}</head>`), 'utf8');
    touched++;
    console.log(`[inject-module-preloads] ${file.replace(DIST, '')}: ${urls.length} preloads`);
  }
}
console.log(`[inject-module-preloads] updated ${touched} HTML file(s)`);
