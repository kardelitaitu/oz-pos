// i18n audit (build gate): check every t() key referenced in source exists
// in both en.json and id.json, and both dicts have identical key sets.
// Missing keys or en/id parity drift EXIT 1 (fails `npm run prebuild`).
// Unused keys are reported for information only — dynamic keys built from
// template literals (e.g. `docs.categories.${group.category}`) are validated
// by prefix (the static part must resolve to a section in both dicts).
// Run: node scripts/audit-i18n.mjs
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

// Resolve a dotted chain against a dict; returns the value or undefined.
function resolveChain(obj, parts) {
  return parts.reduce((acc, part) => (acc && typeof acc === 'object' ? acc[part] : undefined), obj);
}

const src = collect(root);
// Direct form: t(locale, 'key') or t('en','key').
const reT = /t\(\s*locale\s*,\s*(['"])([^'"]+)\1\s*\)/g;
const reT2 = /t\(\s*(['"])([^'"]+)\1\s*,\s*(['"])([^'"]+)\3\s*\)/g; // t('en','key') literal-locale form
// Ternary / variable forms: t(locale, cond ? 'keyA' : 'keyB') — grab every
// quoted dotted literal inside the call so conditional keys can't slip through.
const reT3 = /t\(\s*locale\s*,\s*([^)]*)\)/g;
const reQuoted = /(['"])([a-zA-Z0-9]+\.[a-zA-Z0-9_.]+)\1/g;
// Template-literal prefixes: t(locale, `docs.categories.${group.category}`)
// — the static prefix before ${ must resolve to a section in both dicts.
const reTpl = /t\(\s*locale\s*,\s*`([^`${]+)\$\{/g;

const referenced = new Set();
const tplPrefixes = new Set();
const problems = [];

for (const file of src) {
  const text = readFileSync(file, 'utf8');
  for (const m of text.matchAll(reT)) referenced.add(m[2]);
  for (const m of text.matchAll(reT2)) referenced.add(m[4]);
  for (const m of text.matchAll(reT3)) {
    for (const q of m[1].matchAll(reQuoted)) referenced.add(q[2]);
  }
  for (const m of text.matchAll(reTpl)) tplPrefixes.add(m[1].replace(/\.$/, ''));
}

for (const key of [...referenced].sort()) {
  if (!enKeys.has(key)) problems.push(`MISSING in en.json: ${key}`);
  if (!idKeys.has(key)) problems.push(`MISSING in id.json: ${key}`);
}

// Template-literal prefixes must resolve to a (non-leaf) section in both dicts.
for (const prefix of [...tplPrefixes].sort()) {
  const parts = prefix.split('.');
  for (const [label, dict] of [['en.json', en], ['id.json', id]]) {
    const val = resolveChain(dict, parts);
    if (typeof val !== 'object' || val === null) {
      problems.push(`TEMPLATE PREFIX missing or not a section in ${label}: ${prefix}`);
    }
  }
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
  console.log('\nPROBLEMS (failing build):');
  for (const p of problems) console.log('  ' + p);
  process.exitCode = 1;
} else {
  console.log('parity + presence: OK');
}
if (unused.length) {
  console.log('\nUNUSED (informational — dynamic keys may be false positives):');
  for (const u of unused) console.log('  ' + u);
}
