// Quick internal-link audit of the built dist/ output.
// Run: node scripts/check-links.mjs   (after `npm run build`)
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join, posix } from 'node:path';

function walkAll(dir, out = []) {
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) walkAll(p, out);
    else out.push(p);
  }
  return out;
}

const toUrl = (p) => '/' + p.slice('dist'.length).replaceAll('\\', '/').replace(/^\//, '');

const all = walkAll('dist');
const pages = all.filter((p) => p.endsWith('.html'));
const ok = new Set();
for (const p of all) {
  if (p.endsWith('.html')) {
    const rel = toUrl(p).replace(/index\.html$/, '');
    ok.add(rel);
    ok.add(rel.replace(/\/$/, ''));
  } else {
    ok.add(toUrl(p));
  }
}

const broken = [];
for (const p of pages) {
  const html = readFileSync(p, 'utf8');
  const pageUrl = toUrl(p);
  const pageDir = posix.dirname(pageUrl) + '/'; // e.g. /en/docs/welcome/ -> /en/docs/welcome/
  const re = /href="([^"]+)"/g;
  let m;
  while ((m = re.exec(html))) {
    let h = m[1];
    if (h.startsWith('#') || h.startsWith('http') || h.startsWith('mailto:') || h.startsWith('data:')) continue;
    h = h.split(/[#?]/)[0];
    if (!h) continue;
    const abs = h.startsWith('/')
      ? posix.normalize(h)
      : posix.normalize(posix.join(pageDir, h));
    if (abs === '/') continue;
    if (!ok.has(abs) && !ok.has(abs.replace(/\/$/, ''))) {
      broken.push(`${pageUrl} -> ${m[1]} (resolved ${abs})`);
    }
  }
}

console.log('pages checked:', pages.length, '· files in dist:', all.length);
console.log(broken.length ? 'BROKEN LINKS:\n' + broken.join('\n') : 'NO BROKEN INTERNAL LINKS');
