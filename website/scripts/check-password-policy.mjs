// Password-policy drift guard (build gate). Two jobs:
//
//  1. POLICY PARITY — the client policy module (src/lib/passwordPolicy.ts)
//     must agree with the shared fixture scripts/password-policy-cases.json
//     on every case AND on the policy constants. The Go suite runs the
//     SAME fixture in TestPasswordPolicyMatchesSharedFixture, so if the
//     server or the client changes its notion of a valid password without
//     updating the fixture, the check on that side fails. The two sides
//     can never drift apart silently.
//
//  2. CONFIRM WIRING — every UI create/change password flow must send
//     password_confirm alongside password to the license server
//     (register, set-password, reset-password). If someone removes the
//     confirm field from a component, this check fails the build; the
//     server enforces the same rule (web_password.go) and its Go tests
//     pin the 400-on-mismatch behavior.
//
// Run via `npm run precheck` / `npm run prebuild` (wired in package.json).
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

// Import the ACTUAL shipped policy (Node 22.18+ / 24 strips TS types) —
// not a copy, so the check validates what the meter really uses.
import {
  isStrongPassword,
  passwordClassCount,
  passwordMaxBytes,
  passwordMinClasses,
  passwordMinLen,
} from '../src/lib/passwordPolicy.ts';

const root = join(import.meta.dirname, '..', '..');
const fixture = JSON.parse(
  readFileSync(join(root, 'scripts', 'password-policy-cases.json'), 'utf8'),
);

const problems = [];

// ── 1. Policy constants ──────────────────────────────────────────────
const constChecks = [
  [fixture.minLength, passwordMinLen, 'minLength'],
  [fixture.maxBytes, passwordMaxBytes, 'maxBytes'],
  [fixture.minClasses, passwordMinClasses, 'minClasses'],
];
for (const [fixtureVal, moduleVal, name] of constChecks) {
  if (fixtureVal !== moduleVal) {
    problems.push(`POLICY: fixture ${name}=${fixtureVal} but passwordPolicy.ts exports ${moduleVal}`);
  }
}

// ── 1. Policy cases ─────────────────────────────────────────────────
if (!Array.isArray(fixture.cases) || fixture.cases.length === 0) {
  problems.push('POLICY: fixture has no cases array');
} else {
  for (const c of fixture.cases) {
    if (typeof c.password !== 'string') {
      problems.push(`POLICY: case missing password string: ${JSON.stringify(c)}`);
      continue;
    }
    const gotClasses = passwordClassCount(c.password);
    const gotValid = isStrongPassword(c.password);
    if (gotClasses !== c.classes) {
      problems.push(`POLICY: passwordClassCount(${JSON.stringify(c.password)}) = ${gotClasses}, fixture says ${c.classes}`);
    }
    if (gotValid !== c.valid) {
      problems.push(`POLICY: isStrongPassword(${JSON.stringify(c.password)}) = ${gotValid}, fixture says ${c.valid}`);
    }
  }
}

// ── 2. Confirm wiring ────────────────────────────────────────────────
// Each endpoint's fetch body must carry BOTH password and password_confirm.
const confirmTargets = [
  { file: 'SignupForm.tsx', endpoint: '/api/v1/web/register' },
  { file: 'AuthForm.tsx', endpoint: '/api/v1/web/reset-password' },
  { file: 'AccountView.tsx', endpoint: '/api/v1/web/set-password' },
];
for (const { file, endpoint } of confirmTargets) {
  const text = readFileSync(join(root, 'website', 'src', 'components', file), 'utf8');
  if (!text.includes(endpoint)) {
    problems.push(`CONFIRM: ${file} no longer references ${endpoint}`);
    continue;
  }
  // The endpoint appears in the fetch URL, before the body — scan the
  // window that must contain the JSON.stringify payload.
  const window = text.slice(text.indexOf(endpoint), text.indexOf(endpoint) + 900);
  const body = window.match(/JSON\.stringify\(\s*\{([^}]*)\}\)/s);
  if (!body) {
    problems.push(`CONFIRM: ${file} has no JSON.stringify body near ${endpoint}`);
    continue;
  }
  if (!body[1].includes('password')) {
    problems.push(`CONFIRM: ${file} ${endpoint} body no longer sends password`);
  }
  if (!body[1].includes('password_confirm')) {
    problems.push(`CONFIRM: ${file} ${endpoint} body no longer sends password_confirm`);
  }
}

if (problems.length) {
  console.log(`password-policy check: ${problems.length} problem(s) — failing build:`);
  for (const p of problems) console.log('  ' + p);
  process.exit(1);
}
console.log(
  `password-policy check: OK (${fixture.cases.length} fixture cases match passwordPolicy.ts, ${confirmTargets.length} confirm wirings present)`,
);
