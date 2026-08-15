// Temporary audit: check every t() key referenced in source exists in both
// en.json and id.json, both dicts have identical key sets, and report
// unused keys. Run: node scripts/audit-i18n.mjs
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join, extname } from 'node:path';

const root = join(import.meta.dirname, '..', 'src');

const en = JSON.parse(readFileSync(join(root, 'i18n', 'en.json'), 'utf8'));
const id = JSON.parse(readFileSync(join(root, 'i18n', 'id.json'), 'utf8'));

function flatKeys(obj, prefix = '') {
  return Object.entries(obj).flatMap(([k, v]) => {
    const p = prefix ? `${prefix}.${k}` : k;
    return v && typeof v === 'object' && !Array.isArray(v) ? flatKeys(v, p) : [p];
  });
}

const enKeys = new Set(flatKeys(en));
const idKeys = new Set(flatKeys(id));

// Collect files (excluding node_modules/dist/.astro)
function collect(dir) {
  return readdirSync(dir).flatMap((name) => {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) return collect(p);
    return ['.astro', '.ts', '.tsx'].includes(extname(p)) ? [p] : [];
  });
}

const src = collect(root);
const reT = /t\(\s*locale\s*,\s*'([^']+)'\s*\)/g;
const reT2 = /t\(\s*'([^']+)'\s*,\s*'([^']+)'\s*\)/g; // t('en','key') literal-locale form
const referenced = new Set();
const problems = [];

for (const file of src) {
  const text = readFileSync(file, 'utf8');
  for (const m of text.matchAll(reT)) referenced.add(m[1]);
  for (const m of text.matchAll(reT2)) referenced.add(m[2]);
}

for (const key of [...referenced].sort()) {
  if (!enKeys.has(key)) problems.push(`MISSING in en.json: ${key}`);
  if (!idKeys.has(key)) problems.push(`MISSING in id.json: ${key}`);
}

// Also find dict(locale).x.y access chains (structured data)
const reDict = /dict\(locale\)\.([A-Za-z0-9_.]+)/g;
for (const file of src) {
  const text = readFileSync(file, 'utf8');
  for (const m of text.matchAll(reDict)) {
    const chain = m[1].split('.').filter(Boolean);
    const key = chain.join('.');
    // check the top-level section exists in both
    if (!(chain[0] in en)) problems.push(`dict section missing in en.json: ${chain[0]}`);
    if (!(chain[0] in id)) problems.push(`dict section missing in id.json: ${chain[0]}`);
    void key;
  }
}

// Parity
for (const k of enKeys) if (!idKeys.has(k)) problems.push(`PARITY: en-only key: ${k}`);
for (const k of idKeys) if (!enKeys.has(k)) problems.push(`PARITY: id-only key: ${k}`);

// Unused keys (referenced set)
const unused = [...enKeys].filter((k) => !referenced.has(k)).sort();

console.log(`referenced keys: ${referenced.size}`);
console.log(`en keys: ${enKeys.size}, id keys: ${idKeys.size}`);
if (problems.length) {
  console.log('\nPROBLEMS:');
  for (const p of problems) console.log('  ' + p);
} else {
  console.log('parity + presence: OK');
}
console.log('\nUNUSED (referenced nowhere):');
for (const u of unused) console.log('  ' + u);
