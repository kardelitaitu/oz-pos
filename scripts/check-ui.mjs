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
 *   4. i18n lint     — Fluent key consistency check
 *   5. FTL dedupe    — detect duplicate Fluent keys
 *   6. E2E tests     — Playwright (SKIPPED if Docker is unavailable)
 */

import { execSync } from 'child_process';
import { fileURLToPath } from 'url';
import { dirname, resolve } from 'path';

// ── Change to ui/ directory so we can run from anywhere ─────────────
const __dirname = dirname(fileURLToPath(import.meta.url));
const uiDir = resolve(__dirname, '..', 'ui');
process.chdir(uiDir);

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

  // ── 6. E2E tests (optional — requires Docker) ──────────────────────────
  if (dockerAvailable()) {
    gate('E2E tests (Playwright)', 'npm run test:e2e', { timeout: 600_000 });
  } else {
    console.log(`  ${YELLOW}SKIP (Docker not available)${NC}`);
    results.push({ gate: 'E2E tests (Playwright)', status: 'skip', duration: '0.0' });
  }

  // ── Summary ────────────────────────────────────────────────────────────
  const totalSec = ((Date.now() - totalStart) / 1000).toFixed(1);
  const pass = results.filter((r) => r.status === 'pass').length;
  const skip = results.filter((r) => r.status === 'skip').length;
  const fail = results.filter((r) => r.status === 'fail').length;

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

  if (fail > 0) {
    console.error(`\n${RED}${BOLD}✘ Some checks failed. Fix the issues above and re-run.${NC}\n`);
    process.exit(1);
  }

  console.log(`\n${GREEN}${BOLD}✔ All checks passed${NC}\n`);
}

main();
