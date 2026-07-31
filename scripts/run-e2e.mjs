#!/usr/bin/env node
/**
 * scripts/run-e2e.mjs — Unified E2E test runner.
 *
 * Orchestrates the full E2E test suite cross-platform (no bash dependency):
 *   1. Start Docker backend (cloud server, license server, Redis)
 *   2. Start Vite dev server (with Tauri IPC mock)
 *   3. Run Playwright tests
 *   4. Cleanup and report results
 *
 * Usage:
 *   cd ui && npm run e2e                    # full suite
 *   cd ui && npm run e2e:headed             # watch browser
 *   cd ui && npm run e2e:api                # API tests only
 *   cd ui && npm run e2e:ui                 # UI tests only
 *   cd ui && npm run e2e -- --no-docker     # skip Docker (use existing servers)
 *   cd ui && npm run e2e -- e2e/auth.spec.ts  # single spec
 */

import { execSync, spawn } from 'child_process';
import { generateKeyPairSync } from 'crypto';
import { fileURLToPath } from 'url';
import { dirname, resolve } from 'path';
import { platform } from 'os';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, '..');
const UI_DIR = resolve(ROOT, 'ui');

// Parse args
const args = process.argv.slice(2);
const HEADED = args.includes('--headed');
const API_ONLY = args.includes('--api-only');
const UI_ONLY = args.includes('--ui-only');
const NO_DOCKER = args.includes('--no-docker');
const CHANGED_ONLY = args.includes('--changed-only');
const SPEC_FILES = args.filter(a => !a.startsWith('-'));

/* ── ANSI helpers ───────────────────────────────────────────────────── */
const GREEN  = '\x1b[32m';
const RED    = '\x1b[31m';
const YELLOW = '\x1b[33m';
const CYAN   = '\x1b[36m';
const BOLD   = '\x1b[1m';
const NC     = '\x1b[0m';

/* ── State ──────────────────────────────────────────────────────────── */
let viteProcess = null;
let dockerStarted = false;
let cleanupDone = false;

function log(label, msg) {
  const ts = new Date().toISOString().slice(11, 19);
  console.log(`  ${CYAN}[${ts}]${NC} ${label} ${msg}`);
}

/** Kill a process by its listening port (cross-platform). */
function killByPort(port) {
  try {
    if (platform() === 'win32') {
      // Windows: netstat + taskkill
      const result = execSync(
        `netstat -ano | findstr "LISTENING" | findstr ":${port}"`,
        { stdio: 'pipe', timeout: 5_000 },
      ).toString();
      const lines = result.trim().split('\n');
      for (const line of lines) {
        const parts = line.trim().split(/\s+/);
        const pid = parts[parts.length - 1];
        if (pid && pid !== '0') {
          try { execSync(`taskkill /F /PID ${pid}`, { stdio: 'pipe', timeout: 3_000 }); } catch {}
        }
      }
    } else {
      // Unix: lsof + kill
      execSync(`lsof -ti:${port} | xargs kill -9 2>/dev/null`, { stdio: 'pipe', timeout: 5_000 });
    }
  } catch {
    // No process found on that port — fine
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

/**
 * Ensure OZ_LICENSE_PRIVATE_KEY is set for the license server.
 *
 * If the user has already set OZ_LICENSE_PRIVATE_KEY in their environment,
 * use it as-is (honouring explicit configuration). Otherwise, generate a
 * throwaway RSA-2048 key pair with Node's built-in crypto module so the
 * E2E license server container can boot. The generated key is for testing
 * only — it is not committed, never persisted, and discarded on cleanup.
 *
 * NOTE: The generated key does NOT match the committed public key
 * (crates/oz-core/oz-license.key.pub). Tests that verify real license
 * signatures must provide a real OZ_LICENSE_PRIVATE_KEY instead.
 */
function ensureLicenseKey() {
  if (process.env.OZ_LICENSE_PRIVATE_KEY) {
    log('License', 'Using OZ_LICENSE_PRIVATE_KEY from environment.');
    return;
  }
  log('License', 'Generating throwaway RSA-2048 key for E2E license server...');
  const { privateKey } = generateKeyPairSync('rsa', {
    modulusLength: 2048,
    publicKeyEncoding: { type: 'spki', format: 'der' },
    privateKeyEncoding: { type: 'pkcs8', format: 'pem' },
  });
  // Escape literal newlines to \n so the PEM survives shell/env boundaries
  // intact. The Go license server's normalizePEM() reverses this on receipt.
  process.env.OZ_LICENSE_PRIVATE_KEY = privateKey.replace(/\n/g, '\\n');
  log('License', 'Throwaway key generated (valid for this session only).');
}

/** Start Docker E2E services. */
function startDocker() {
  log('Docker', 'Starting E2E services...');
  execSync(
    `docker compose -f "${ROOT}/docker-compose.e2e.yml" up -d --wait`,
    { stdio: 'inherit', timeout: 120_000 },
  );
  dockerStarted = true;
  log('Docker', 'Services ready.');
}

/** Stop Docker E2E services. */
function stopDocker() {
  if (!dockerStarted) return;
  log('Docker', 'Stopping E2E services...');
  try {
    execSync(
      `docker compose -f "${ROOT}/docker-compose.e2e.yml" down -v`,
      { stdio: 'pipe', timeout: 60_000 },
    );
    dockerStarted = false;
    log('Docker', 'Stopped.');
  } catch (e) {
    log('Docker', `Cleanup warning: ${e.message.slice(0, 80)}`);
  }
}

/** Start Vite dev server, return when ready. */
function startVite() {
  return new Promise((resolvePromise, reject) => {
    // Check if Vite is already running
    try {
      execSync('curl -sf http://localhost:1420 > /dev/null 2>&1', { timeout: 3_000 });
      log('Vite', 'Already running on port 1420, reusing.');
      resolvePromise();
      return;
    } catch {
      // Not running — kill any stale process on port 1420, then start
    }

    killByPort(1420);

    log('Vite', 'Starting dev server...');
    viteProcess = spawn('npx', ['vite'], {
      cwd: UI_DIR,
      stdio: ['ignore', 'pipe', 'pipe'],
      shell: true,
    });

    let outputBuffer = '';
    const timeout = setTimeout(() => {
      reject(new Error('Vite dev server failed to start within 60s'));
    }, 60_000);

    const onData = (data) => {
      outputBuffer += data.toString();
      if (outputBuffer.includes('Local:') || outputBuffer.includes('localhost:1420')) {
        clearTimeout(timeout);
        log('Vite', 'Ready.');
        resolvePromise();
      }
    };

    viteProcess.stdout.on('data', onData);
    viteProcess.stderr.on('data', onData);

    viteProcess.on('error', (err) => {
      clearTimeout(timeout);
      reject(err);
    });

    viteProcess.on('exit', (code) => {
      clearTimeout(timeout);
      if (code !== 0 && !outputBuffer.includes('ready')) {
        reject(new Error(`Vite exited with code ${code}`));
      }
    });
  });
}

/** Stop Vite dev server. */
function stopVite() {
  if (viteProcess) {
    log('Vite', 'Stopping dev server...');
    viteProcess.kill('SIGTERM');
    viteProcess = null;
  }
  killByPort(1420);
}

/**
 * Get the list of E2E spec files changed relative to the default branch.
 * Returns an empty array if git detection fails or there's no diff.
 */
function getChangedSpecs() {
  try {
    // Try origin/main first, then origin/master, then main
    const refs = ['origin/main', 'origin/master', 'main'];
    let mergeBase;
    for (const ref of refs) {
      try {
        mergeBase = execSync(
          `git merge-base HEAD "${ref}"`,
          { stdio: 'pipe', timeout: 10_000, cwd: ROOT },
        ).toString().trim();
        if (mergeBase) break;
      } catch {}
    }
    if (!mergeBase) return [];

    // git diff --name-only outputs paths relative to repo root,
    // so we run from ROOT with pattern ui/e2e/*.spec.ts to get
    // results like "ui/e2e/auth.spec.ts", then strip ui/ prefix.
    const changed = execSync(
      `git diff --name-only "${mergeBase}" -- 'ui/e2e/*.spec.ts'`,
      { stdio: 'pipe', timeout: 10_000, cwd: ROOT },
    ).toString().trim().split('\n').filter(Boolean);

    return changed.map(f => f.replace(/^ui\//, 'e2e/'));
  } catch {
    return [];
  }
}

/** Run Playwright tests. */
function runPlaywright() {
  let cmd = `npx playwright test --config e2e/playwright.config.ts`;

  if (HEADED) cmd += ' --headed';

  // Determine which spec files to run
  let specs = SPEC_FILES;

  // --changed-only overrides explicit spec files when none given
  if (specs.length === 0 && CHANGED_ONLY) {
    const changed = getChangedSpecs();
    if (changed.length > 0) {
      log('Playwright', `Changed-only: ${changed.length} spec(s) modified`);
      specs = changed;
    } else {
      log('Playwright', `Changed-only: no E2E specs changed — skipping.`);
      return true; // Nothing to test = pass
    }
  }

  if (specs.length === 0) {
    if (API_ONLY) {
      specs = ['e2e/api.spec.ts'];
    } else if (UI_ONLY) {
      specs = [
        'e2e/auth.spec.ts',
        'e2e/sale.spec.ts',
        'e2e/pos-workflows.spec.ts',
        'e2e/product.spec.ts',
        'e2e/shift.spec.ts',
        'e2e/settings.spec.ts',
        'e2e/new-flows.spec.ts',
        'e2e/e2e-sale-to-history.spec.ts',
        'e2e/e2e-shift-reconciliation.spec.ts',
        'e2e/e2e-settings-persist.spec.ts',
      ];
    }
  }

  if (specs.length > 0) {
    cmd += ' ' + specs.map(f => f.startsWith('e2e/') ? f : `e2e/${f}`).join(' ');
  }

  log('Playwright', `Running: ${cmd}`);

  try {
    execSync(cmd, { cwd: UI_DIR, stdio: 'inherit', timeout: 600_000 });
    log('Playwright', `${GREEN}All tests passed${NC}`);
    return true;
  } catch {
    log('Playwright', `${RED}Some tests failed${NC}`);
    return false;
  }
}

/** Cleanup everything. */
function cleanup() {
  if (cleanupDone) return;
  cleanupDone = true;
  console.log();

  stopVite();

  if (!NO_DOCKER) {
    stopDocker();
  }

  log('Cleanup', 'Done.');
}

/* ── Main ───────────────────────────────────────────────────────────── */
async function main() {
  // Register cleanup for Ctrl+C and normal exit
  process.on('SIGINT', () => { cleanup(); process.exit(1); });
  process.on('SIGTERM', () => { cleanup(); process.exit(1); });

  let allPassed = false;

  try {
    console.log(`\n${BOLD}${CYAN}═══════════════════════════════════════${NC}`);
    console.log(`${BOLD}${CYAN}  OZ-POS — E2E Test Suite${NC}`);
    console.log(`${BOLD}${CYAN}═══════════════════════════════════════${NC}\n`);

    // ── Step 1: Start Docker backend ──────────────────────────────
    if (!NO_DOCKER) {
      if (dockerAvailable()) {
        ensureLicenseKey();
        startDocker();
      } else {
        log('Docker', `${YELLOW}Not available — skipping.${NC}`);
      }
    } else {
      log('Docker', 'Skipped (--no-docker).');
    }

    // ── Step 2: Start Vite dev server ─────────────────────────────
    await startVite();

    // ── Step 3: Run Playwright tests ──────────────────────────────
    allPassed = runPlaywright();

  } catch (err) {
    console.error(`\n${RED}${BOLD}✘ Error:${NC} ${err.message || err}`);
  } finally {
    cleanup();
  }

  console.log(`\n${BOLD}${CYAN}═══════════════════════════════════════${NC}`);
  if (allPassed) {
    console.log(`${GREEN}${BOLD}✔ All E2E tests passed${NC}`);
    process.exit(0);
  } else {
    console.log(`${RED}${BOLD}✘ Some E2E tests failed — check output above${NC}`);
    process.exit(1);
  }
}

main();
