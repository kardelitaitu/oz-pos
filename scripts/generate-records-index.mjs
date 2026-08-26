// ── generate-records-index.mjs ─────────────────────────────────────────────
// Regenerates docs/records/README.md — the unified engineering-records
// registry — from the record files themselves, so the index never drifts
// as ADRs/audits are added.
//
// Each record contributes:
//   title   — first `#` heading (or YAML front-matter `title:` if present)
//   status  — `**Status:**`/`> **Status:**`/`Status:` line (or front-matter)
//   area    — derived from the filename slug keywords (see AREA_KEYWORDS),
//             overridable via YAML front-matter `area:`
//
// Usage:  node scripts/generate-records-index.mjs
// Output: docs/records/README.md (header + conventions are regenerated too)

import { readdirSync, readFileSync, writeFileSync, existsSync } from 'node:fs';
import { join, relative, basename, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const RECORDS = join(ROOT, 'docs', 'records');
const OUT = join(RECORDS, 'README.md');

// ── area keyword map (checked against the lowercase filename slug) ────────
// First match wins; order matters (most specific first).
const AREA_KEYWORDS = [
  ['research', ['research', 'forecasting', 'voice']],
  ['subscription', ['subscription', 'tier', 'trial', 'license', 'midtrans', 'paddle', 'billing']],
  ['payments', ['payment', 'stripe', 'qris', 'cash', 'settlement', 'gateway']],
  ['sync', ['sync', 'crdt', 'conflict', 'offline']],
  ['topology', ['topology', 'node-based', 'node-', 'warehouse-allocation', 'stock-routing', 'connection-gating', 'typed-connection', 'multi-terminal', 'peer-model']],
  ['inventory', ['inventory', 'warehouse', 'location', 'multi-location', 'stock']],
  ['kds', ['kds', 'kitchen']],
  ['loyalty', ['loyalty']],
  ['staff', ['staff', 'rbac', 'role', 'user-profile']],
  ['products', ['product', 'category', 'rack', 'attributes', 'popularity', 'context-menu']],
  ['money', ['money', 'currency', 'exchange', 'rounding', 'tax-rounding']],
  ['tax', ['tax', 'ppn', 'pb1']],
  ['reporting', ['reporting', 'report', 'analytics']],
  ['accessibility', ['accessibility', 'a11y', 'keyboard-shortcuts']],
  ['theming', ['theme', 'theming', 'shadow', 'css', 'whitelabel', 'branding']],
  ['ui', ['loading-states', 'empty-states', 'modal', 'ui-state', 'frontend', 'react', 'ux', 'tablet', 'table-management', 'dev-mock']],
  ['crm', ['crm', 'customer']],
  ['module-system', ['module-system', 'event-bus', 'module-registration', 'domain-module']],
  ['architecture', ['architecture', 'frontend-restructure', 'data-scope', 'panic-policy', 'tenancy', 'workspace-type', 'rust-backend']],
  ['security', ['audit-log', 'security', 'auth']],
  ['performance', ['performance']],
  ['plugin', ['plugin']],
  ['quality', ['code-quality', 'dev-experience', 'coverage']],
  ['release', ['release', 'ci', 'docker', 'updater', 'migration', 'deploy', 'vps']],
  ['database', ['database', 'migration', 'db-', 'sql']],
  ['observability', ['logging', 'error-handling', 'observability', 'diagnostics']],
  ['general', []],
];

function areaFromSlug(slug) {
  for (const [area, keys] of AREA_KEYWORDS) {
    if (keys.some((k) => slug.includes(k))) return area;
  }
  return 'general';
}

// ── tiny YAML-ish front-matter parser (title/status/area keys) ─────────────
function frontMatter(file) {
  const text = file.split(/[\r\n]+/);
  if (text[0] !== '---') return null;
  let i = 1;
  const out = {};
  while (i < text.length && text[i] !== '---') {
    const m = text[i].match(/^([A-Za-z_-]+):\s*(.*)$/);
    if (m) out[m[1].toLowerCase()] = m[2].trim();
    i++;
  }
  return out;
}

function extractTitle(file, text) {
  const fm = frontMatter(file);
  if (fm?.title) return fm.title;
  const m = text.match(/^#\s+(.+)$/m);
  return m ? m[1].replace(/\\r/g, '').trim() : basename(file).replace(/\.md$/, '');
}

function extractStatus(text) {
  const fm = frontMatter(text);
  if (fm?.status) return fm.status.replace(/\\r/g, '').trim();
  // Prefer the `> **Status:**` (audits) then `**Status:**` (ADRs), then bare `Status:`
  for (const re of [
    /^>\s*\*\*Status:\*\*\s*(.+)$/m,
    /^\*\*Status:\*\*\s*(.+)$/m,
    /^Status:\s*(.+)$/m,
  ]) {
    const m = text.match(re);
    if (m) return m[1].replace(/\\r/g, '').trim();
  }
  return '—';
}

function readRecord(filePath) {
  const text = readFileSync(filePath, 'utf8').replace(/\r/g, ''); // normalize CRLF
  const fm = frontMatter(text);
  const slug = basename(filePath).replace(/\.md$/, '').toLowerCase();
  return {
    file: filePath,
    slug: basename(filePath),
    title: extractTitle(filePath, text),
    status: extractStatus(text),
    area: fm?.area ?? areaFromSlug(slug),
  };
}

// ── ADR number mapping (filename → registry #) ─────────────────────────────
// Kept here because several ADRs predate the `adrNN` filename convention and
// carry their number only in the title. Update when a new numbered ADR lands.
const ADR_NUMBERS = {
  '2026-01-15-module-system-design.md': 1,
  '2026-02-01-event-bus-design.md': 2,
  '2026-03-01-frontend-restructure.md': 3,
  '2026-07-10-workspace-type-instance-design.md': 4,
  '2026-07-10-subscription-tier-entitlement.md': 5,
  '2026-07-10-crdt-delta-ledger-offline-sync.md': 6,
  '2026-07-10-data-scope-guard.md': 7,
  '2026-07-10-scoped-event-bus.md': 8,
  '2026-07-10-license-server.md': 9,
  '2026-07-13-sync-performance-compression-batching.md': 10,
  '2026-07-13-zero-downtime-vps-migration.md': 11,
  '2026-07-15-whitelabel-branding-system.md': 12,
  '2026-07-16-desktop-app-updater.md': 13,
  '2026-07-16-release-automation.md': 14,
  '2026-07-18-shadow-banding-css-dither.md': 15,
  '2026-07-18-kds-multi-layout-system.md': 17,
  '2026-07-18-multi-location-inventory.md': 18,
  '2026-07-19-sale-deduction-multi-location.md': 19,
  '2026-07-19-payment-capture-ordering.md': 20,
  '2026-07-20-sync-conflict-resolution-strategy.md': 21,
  '2026-07-20-node-based-store-topology-builder.md': 22,
  '2026-07-20-free-trial-lifecycle-and-license-activation-workflow.md': 23,
  '2026-07-24-domain-module-extraction.md': 30,
  '2026-07-24-react-only-decision.md': 30,
  '2026-07-24-decentralized-ui-module-registration.md': 31,
  '2026-07-25-db-extraction-and-platform-split.md': 32,
  '2026-08-03-panic-policy.md': 33,
  '2026-08-07-business-logic-topology-builder.md': 34,
  '2026-08-08-adr34-typed-connection-gating.md': 34,
  '2026-08-11-adr35-rbac-role-assignments-user-profile.md': 35,
  '2026-08-11-adr36-retail-product-attributes.md': 36,
  '2026-08-11-adr37-product-popularity-index.md': 37,
  '2026-08-11-adr38-retail-row-context-menu-browser-images.md': 38,
  '2026-08-18-adr39-midtrans-subscription-payments.md': 39,
  '2026-08-20-adr40-multi-terminal-peer-model.md': 40,
};

const md = (p) => p.replace(/\\/g, '/');

function relFromRecords(filePath) {
  return md(relative(RECORDS, filePath));
}

// ── build sections ─────────────────────────────────────────────────────────
const decisionsDir = join(ROOT, 'docs', 'decisions');
const auditDir = join(ROOT, 'audit');
const observabilityDir = join(ROOT, 'docs', 'observability');

const numbered = [];
const research = [];
const phases = [];
const audits = [];
const observability = [];

if (existsSync(decisionsDir)) {
  for (const f of readdirSync(decisionsDir).filter((f) => f.endsWith('.md'))) {
    if (f === 'README.md' || f.endsWith('.status.md')) continue;
    const rec = readRecord(join(decisionsDir, f));
    const num = ADR_NUMBERS[f];
    if (num) {
      const statusFile = join(decisionsDir, f.replace(/\.md$/, '.status.md'));
      const statusLink = existsSync(statusFile)
        ? `${rec.status} (see [status](./${f.replace(/\.md$/, '.status.md')}))`
        : rec.status;
      numbered.push({ ...rec, num, status: statusLink });
    } else if (f.includes('research')) {
      research.push(rec);
    } else {
      phases.push(rec);
    }
  }
}
numbered.sort((a, b) => a.num - b.num);

// ── audit section ──────────────────────────────────────────────────────────
// After the sector reports were consolidated into docs/records/audit-open-findings.md,
// the registry points at that summary instead of the per-sector files. If the
// `audit/` folder still exists (e.g. mid-migration), list its files; otherwise
// emit the pointer to the consolidated summary.
if (existsSync(auditDir)) {
  for (const f of readdirSync(auditDir).filter((f) => f.endsWith('.md'))) {
    if (f === 'AUDIT_JULY_2026.md') continue;
    const rec = readRecord(join(auditDir, f));
    const num = parseInt(rec.slug, 10);
    audits.push({ ...rec, num: Number.isNaN(num) ? null : num });
  }
  audits.sort((a, b) => (a.num ?? 99) - (b.num ?? 99));
}

if (existsSync(observabilityDir)) {
  for (const f of readdirSync(observabilityDir).filter((f) => f.endsWith('.md'))) {
    observability.push(readRecord(join(observabilityDir, f)));
  }
}

// ── scattered docs list (kept explicit — they have no folder pattern) ──────
const scattered = [
  'docs/2026-07-28-retail-pos-theming-audit.md',
  'docs/2026-07-29-retail-pos-ux-audit.md',
  'docs/code-quality-2026-07-20.md',
  'docs/database-optimization-2026-07-20.md',
  'docs/dev-experience-2026-07-20.md',
  'docs/dev-mock-state-audit.md',
  'docs/ui-state-audit-2026-07-20.md',
  'docs/modal-audit-checklist.md',
  'docs/TODO-shadow-audit.md',
  'docs/plan-product-images-review.md',
  'docs/design-exceptions.md',
].filter((p) => existsSync(join(ROOT, p))).map((p) => readRecord(join(ROOT, p)));

// ── emit ───────────────────────────────────────────────────────────────────
const L = [];
const row = (cells) => `| ${cells.join(' | ')} |`;

L.push('# Engineering Records');
L.push('');
L.push('> **Generated by [`scripts/generate-records-index.mjs`](../../scripts/generate-records-index.mjs)** — do not edit by hand. Run `node scripts/generate-records-index.mjs` after adding or changing a record.');
L.push('');
L.push('Unified registry for architectural decisions (ADRs), audits, verifications, and system analyses. `area` is derived from the record filename (overridable via YAML front-matter `area:`). Files live at their current paths; this index is the single entry point. Still-open audit findings live in [**Audit Open Findings**](./audit-open-findings.md).');
L.push('');

// ── Numbered ADRs ──
L.push('## Architectural Decision Records (`docs/decisions/`)');
L.push('');
L.push('### Numbered ADRs');
L.push('');
L.push(row(['#', 'Area', 'Title', 'Status']));
L.push(row(['---', '---', '---', '---']));
for (const r of numbered) {
  L.push(row([String(r.num), r.area, `[${r.title}](${relFromRecords(r.file)})`, r.status]));
}
L.push('');

// ── Research notes ──
if (research.length) {
  L.push('### Research Notes');
  L.push('');
  for (const r of research) {
    L.push(`- **${r.area}** — [${r.title}](${relFromRecords(r.file)})`);
  }
  L.push('');
}

// ── Phased implementation docs ──
if (phases.length) {
  L.push('### Phased Implementation Docs');
  L.push('');
  const byArea = {};
  for (const r of phases) {
    (byArea[r.area] ??= []).push(r);
  }
  for (const [area, list] of Object.entries(byArea)) {
    L.push(`**${area}:**`);
    for (const r of list) {
      L.push(`- [${r.title}](${relFromRecords(r.file)})`);
    }
    L.push('');
  }
}

// ── Audits ──
if (audits.length) {
  L.push('## Audit Reports (consolidated)');
  L.push('');
  L.push(row(['#', 'Area', 'Title', 'Status']));
  L.push(row(['---', '---', '---', '---']));
  for (const r of audits) {
    L.push(row([r.num ?? '—', r.area, `[${r.title}](${relFromRecords(r.file)})`, r.status]));
  }
  L.push('');
} else {
  L.push('## Audit Reports');
  L.push('');
  L.push(`The per-sector audit reports were consolidated into [**Audit Open Findings**](./audit-open-findings.md) (${existsSync(join(RECORDS, 'audit-open-findings.md')) ? 'current' : 'generated — run the script again'}); fully-remediated sectors are closed by the commits recorded there.`);
  L.push('');
}

// ── Scattered audit reports ──
L.push('## Scattered Audit Reports (`docs/`)');
L.push('');
for (const r of scattered) {
  L.push(`- **${r.area}** — [${r.title}](${relFromRecords(r.file)})`);
}
L.push('');

// ── Observability ──
L.push('## System Analysis / Observability (`docs/observability/`)');
L.push('');
L.push(row(['Area', 'Title', 'Status']));
L.push(row(['---', '---', '---']));
for (const r of observability) {
  L.push(row([r.area, `[${r.title}](${relFromRecords(r.file)})`, r.status]));
}
L.push('');

// ── Conventions ──
L.push('## Conventions');
L.push('');
L.push('- **ADR naming:** `YYYY-MM-DD-adrNN-<slug>.md` in `docs/decisions/`');
L.push('- **Audit records:** per-sector reports were consolidated into [`audit-open-findings.md`](./audit-open-findings.md); open findings are tracked there');
L.push('- **`area:` tag:** derived from the filename slug (see `AREA_KEYWORDS` in the generator); set `area:` in YAML front-matter to override');
L.push('- **Status vocabulary:** ADRs use *proposed / accepted / implemented / superseded / re-scoped*; audits use *remediated / partially remediated / audited / open*');
L.push('- **Adding a new record:** drop the file in the right folder, then run `node scripts/generate-records-index.mjs`');

writeFileSync(OUT, L.join('\n') + '\n', 'utf8');
console.log(`Wrote ${OUT} (${numbered.length} ADRs, ${research.length} research, ${phases.length} phased, ${audits.length} audits, ${scattered.length} scattered, ${observability.length} observability)`);
