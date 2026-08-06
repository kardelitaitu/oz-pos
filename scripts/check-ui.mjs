#!/usr/bin/env node
/**
 * scripts/check-ui.mjs — Unified UI validation runner.
 *
 * Chains all UI validation gates into one `npm run check:all` command.
 * Mirrors the UI portion of scripts/check.sh but runs cross-platform
 * (no bash dependency).
 *
 * Usage:  cd ui && npm run check:all
 *
 * Gates (in order):
 *   1. Lint          — ESLint (jsx-a11y, react-hooks)
 *   2. TypeScript    — tsc --noEmit (strict type checking)
 *   3. Unit tests    — vitest run (214 files, 3230+ tests)
 *   4. i18n lint     — Fluent key consistency check *  5. FTL dedupe    — detect duplicate Fluent keys
 *  6. Bundle budget — gzip budgets on the production build (PERF-02)
 *  7. E2E tests     — Playwright (SKIPPED if Docker is unavailable)
 */

import { execSync } from 'child_process';
import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, resolve } from 'path';

// ── Change to ui/ directory so we can run from anywhere ─────────────
const __dirname = dirname(fileURLToPath(import.meta.url));
const uiDir = resolve(__dirname, '..', 'ui');
process.chdir(uiDir);

// ── Gate manifest (AUDIT-27 CI-08) ──────────────────────────────────
// The `check:all` gate vocabulary derives from scripts/gates.json (the
// single source of truth shared with ci.yml, nightly.yml, check.sh, and
// the CI docs-drift verifier). If a manifest `check:all` gate is not
// declared below, this runner must fail closed — exactly like the CI
// `ci-docs-drift` gate does.
const gatesManifestPath = resolve(__dirname, 'gates.json');
let manifestCheckAllNeedles = [];
let manifestReadable = true;
try {
  const manifest = JSON.parse(readFileSync(gatesManifestPath, 'utf8'));
  manifestCheckAllNeedles = (manifest.gates ?? [])
    .map((g) => g.runners?.['check:all'] ?? [])
    .flat()
    .map((n) => n.toLowerCase());
} catch {
  manifestReadable = false;
}

/* ── ANSI helpers ───────────────────────────────────────────────────── */
const GREEN  = '\x1b[32m';
const RED    = '\x1b[31m';
const YELLOW = '\x1b[33m';
const CYAN   = '\x1b[36m';
const BOLD   = '\x1b[1m';
const NC     = '\x1b[0m'; // reset

/* ── State ──────────────────────────────────────────────────────────── */
const results = []; // { gate, status, duration }

/**
 * Run a single validation gate.
 *
 * @param {string}  name         Human-readable gate name.
 * @param {string}  command      Shell command to execute.
 * @param {object}  [opts]
 * @param {number}  [opts.timeout]  Timeout in ms (default 300_000).
 */
function gate(name, command, opts = {}) {
  const timeout = opts.timeout ?? 300_000;
  const start = Date.now();

  process.stdout.write(`  ${CYAN}▶${NC} ${name} ... `);

  try {
    execSync(command, { stdio: 'pipe', timeout });
    const sec = ((Date.now() - start) / 1000).toFixed(1);
    console.log(`${GREEN}PASS (${sec}s)${NC}`);
    results.push({ gate: name, status: 'pass', duration: sec });
  } catch {
    const sec = ((Date.now() - start) / 1000).toFixed(1);
    console.log(`${RED}FAIL (${sec}s)${NC}`);
    results.push({ gate: name, status: 'fail', duration: sec });

    // Re-run with inherited stdio so the user sees the full error output
    console.error(`\n${RED}── ${name} ──${NC}`);
    try {
      execSync(command, { stdio: 'inherit', timeout });
    } catch {
      // ignore — we already know it failed
    }
    console.error();
  }
}

/** Check whether Docker is available (daemon reachable). */
function dockerAvailable() {
  try {
    execSync('docker info', { stdio: 'pipe', timeout: 10_000 });
    return true;
  } catch {
    return false;
  }
}

/** Check whether Playwright browsers are installed (for the perf smoke suite). */
function playwrightAvailable() {
  try {
    execSync('npx playwright --version', { stdio: 'pipe', timeout: 30_000 });
    return true;
  } catch {
    return false;
  }
}

/* ── Main ───────────────────────────────────────────────────────────── */
function main() {
  const totalStart = Date.now();

  console.log(`\n${BOLD}${CYAN}═══════════════════════════════════════${NC}`);
  console.log(`${BOLD}${CYAN}  OZ-POS — UI Validation Gates${NC}`);
  console.log(`${BOLD}${CYAN}═══════════════════════════════════════${NC}\n`);

  // ── 1. Lint ────────────────────────────────────────────────────────────
  gate('ESLint', 'npm run lint');

  // ── 2. TypeScript ──────────────────────────────────────────────────────
  gate('TypeScript type check', 'npm run typecheck');

  // ── 3. Unit tests ──────────────────────────────────────────────────────
  gate('Unit tests (vitest)', 'npm run test', { timeout: 600_000 });

  // ── 4. i18n lint ───────────────────────────────────────────────────────
  gate('i18n lint', 'npm run lint:i18n');

  // ── 5. FTL dedupe ──────────────────────────────────────────────────────
  gate('FTL dedupe', 'npm run dedupe:ftl');

  // ── 6. Bundle budget (PERF-02) — production build + gzip size gates ────
  gate('Bundle budget', 'npm run bundle:check', { timeout: 300_000 });

  // ── 7. E2E tests (optional — requires Docker) ──────────────────────────
  // AUDIT-27 CI-07: use `npm run e2e` (scripts/run-e2e.mjs) which
  // PROVISIONS the Docker backend (cloud + license + redis), starts Vite,
  // runs Playwright, and cleans up — rather than bare `playwright test`
  // which would run against whatever happens to be on port 1420/3099.
  if (dockerAvailable()) {
    gate('E2E tests (Playwright, provisioned)', 'npm run e2e', { timeout: 900_000 });
  } else {
    console.log(`  ${YELLOW}SKIP (Docker not available)${NC}`);
    results.push({ gate: 'E2E tests (Playwright)', status: 'skip', duration: '0.0' });
  }

  // ── 8. Perf smoke suite (PERF-10) — UI-only, no Docker required ────────
  // Runs the Playwright performance smoke suite (desktop + tablet budgets).
  // Skipped when Playwright browsers are not installed locally.
  if (playwrightAvailable()) {
    gate('Perf smoke (Playwright)', 'npm run test:e2e:perf', { timeout: 600_000 });
  } else {
    console.log(`  ${YELLOW}SKIP (Playwright not available)${NC}`);
    results.push({ gate: 'Perf smoke (Playwright)', status: 'skip', duration: '0.0' });
  }

  // ── Summary ────────────────────────────────────────────────────────────
  const totalSec = ((Date.now() - totalStart) / 1000).toFixed(1);
  const pass = results.filter((r) => r.status === 'pass').length;
  const skip = results.filter((r) => r.status === 'skip').length;
  const fail = results.filter((r) => r.status === 'fail').length;

  // ── Gate manifest self-audit (AUDIT-27 CI-08) ─────────────────────
  // The gate vocabulary derives from scripts/gates.json. Every manifest
  // `check:all` needle must match at least one gate this runner actually
  // declared; a manifest gate that is not declared is drift and fails
  // this runner closed — mirroring the CI `ci-docs-drift` gate.
  let manifestOk = true;
  if (manifestReadable) {
    const declared = results.map((r) => r.gate.toLowerCase());
    const missing = manifestCheckAllNeedles.filter(
      (needle) => !declared.some((g) => g.includes(needle))
    );
    if (missing.length > 0) {
      manifestOk = false;
      console.error(`\n${RED}✘ Gate manifest drift: check:all does not declare ${missing.length} manifest gate(s): ${missing.join(', ')}${NC}`);
      console.error(`  Fix scripts/gates.json or add the missing gate here.`);
    }
  } else {
    console.log(`  ${YELLOW}⚠ Gate manifest unreadable — manifest self-audit skipped (${gatesManifestPath})${NC}`);
  }

  console.log(`\n${BOLD}${CYAN}═══════════════════════════════════════${NC}`);
  console.log(`${BOLD}${CYAN}  Summary${NC}`);
  console.log(`${BOLD}${CYAN}═══════════════════════════════════════${NC}`);
  for (const r of results) {
    const icon =
      r.status === 'pass' ? `${GREEN}✔${NC}` :
      r.status === 'skip' ? `${YELLOW}–${NC}` :
                             `${RED}✘${NC}`;
    const label = r.status === 'pass' ? 'Pass' : r.status === 'skip' ? 'Skip' : 'FAIL';
    console.log(`  ${icon} ${r.gate} (${r.duration}s) — ${label}`);
  }
  console.log(`\n  Total: ${totalSec}s  |  ${GREEN}${pass} passed${NC}  ${YELLOW}${skip} skipped${NC}  ${fail > 0 ? `${RED}${fail} failed` : ''}${NC}`);

  if (fail > 0 || !manifestOk) {
    console.error(`\n${RED}${BOLD}✘ Some checks failed. Fix the issues above and re-run.${NC}\n`);
    process.exit(1);
  }

  console.log(`\n${GREEN}${BOLD}✔ All checks passed${NC}\n`);
}

main();
