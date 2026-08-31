#!/usr/bin/env node
/**
 * scripts/import-portal.mjs — stage the built docs portal into website/public/docs-portal
 *
 * On Windows: Uses native robocopy with 32 worker threads and differential sync (/MIR),
 * reducing runtime from 85+ seconds (WSL/MSYS rm -rf of 6,100 files across DrvFS)
 * to ~0.07 seconds when unchanged, and ~2 seconds on full update.
 *
 * On Unix: Uses native fs.cpSync.
 */
import { execFileSync } from 'node:child_process';
import { existsSync, cpSync, rmSync, mkdirSync } from 'node:fs';
import { platform } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const WEBSITE_ROOT = path.resolve(SCRIPT_DIR, '..');
const REPO_ROOT = path.resolve(WEBSITE_ROOT, '..');

const PORTAL_SRC = path.join(REPO_ROOT, 'docs', 'book');
const PORTAL_DEST = path.join(WEBSITE_ROOT, 'public', 'docs-portal');

const ifExists = process.argv.includes('--if-exists');

if (!existsSync(path.join(PORTAL_SRC, 'index.html'))) {
  if (ifExists) {
    console.log(`import-portal: portal not built (${PORTAL_SRC} missing) — skipping (--if-exists)`);
    process.exit(0);
  }
  console.error(`error: portal not built — run 'bash scripts/build-docs.sh' first (expected ${PORTAL_SRC})`);
  process.exit(1);
}

console.log('import-portal: staging portal into website...');

if (platform() === 'win32') {
  try {
    execFileSync('robocopy', [PORTAL_SRC, PORTAL_DEST, '/MIR', '/MT:32', '/R:1', '/W:1', '/NP', '/NDL', '/NFL', '/NJH', '/NJS'], { stdio: 'ignore' });
  } catch (err) {
    // Robocopy exit codes 0-7 indicate success (0 = identical, 1 = copied files)
    if (err.status >= 8) {
      console.error(`error: robocopy failed with status ${err.status}`);
      process.exit(err.status);
    }
  }
} else {
  rmSync(PORTAL_DEST, { recursive: true, force: true });
  mkdirSync(PORTAL_DEST, { recursive: true });
  cpSync(PORTAL_SRC, PORTAL_DEST, { recursive: true });
}

console.log(`✔ portal staged: ${PORTAL_DEST}`);
