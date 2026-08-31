#!/usr/bin/env node
/**
 * scripts/bust-admin-cache.mjs — Stamp the {{VERSION}} placeholder in admin
 * HTML files with a build-unique cache-busting string.
 *
 * The admin console (public/admin/) uses `?v=` query strings so browsers
 * pick up new CSS/JS after a deploy. These were hand-bumped and routinely
 * forgot. The source files carry a `{{VERSION}}` placeholder; this script
 * replaces it with the current git short hash.
 *
 * Usage:
 *   node scripts/bust-admin-cache.mjs          # stamps dist/admin/ (postbuild)
 *   node scripts/bust-admin-cache.mjs public   # stamps public/admin/ (prebuild, dirty)
 *
 * Git is available in CI and local builds; on a broken/absent git we fall
 * back to a timestamp so the build never fails on this step.
 */
import { execSync } from 'node:child_process';
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const ROOT = process.cwd();
const target = process.argv[2] || 'dist';
const FILES = [join(target, 'admin', 'index.html'), join(target, 'admin', 'login.html')];

function buildVersion() {
  try {
    const hash = execSync('git rev-parse --short HEAD', { cwd: ROOT, encoding: 'utf8' }).trim();
    if (hash) return hash;
  } catch {
    // fall through to timestamp
  }
  return Date.now().toString(36);
}

const version = buildVersion();
let changed = 0;
for (const p of FILES) {
  if (!existsSync(p)) continue;
  let text = readFileSync(p, 'utf8');
  if (!text.includes('{{VERSION}}')) continue;
  const next = text.replace(/\{\{VERSION\}\}/g, version);
  writeFileSync(p, next, 'utf8');
  changed += 1;
}

console.log(`[bust-admin-cache] stamped ${version} into ${changed} file(s) (${target}/admin/)`);