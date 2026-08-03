#!/usr/bin/env node
/**
 * scripts/__tests__/pipefail.test.mjs
 *
 * AUDIT-27 CI-04 regression test: every CI step that pipes a test command
 * into `tee` must use a shell that fails on a broken pipe (pipefail), so a
 * failing test before `tee` can never be masked by a successful tee.
 *
 * The workflows declare `shell: bash --noprofile --norc -eo pipefail {0}`
 * on the tee-wrapped steps. This test proves that shell actually
 * propagates a nonzero exit from the left side of the pipe, and that the
 * workflows really declare it on every tee step.
 *
 * Run:  npx node --test scripts/__tests__/pipefail.test.mjs
 */

import { describe, it } from 'node:test';
import assert from 'node:assert';
import { execSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, '..', '..');

/** Run a command under the exact shell flags the workflows declare. */
function runWithPipefailFlags(cmd) {
  // `bash` resolves from PATH (Git Bash on Windows). The inner pipe runs
  // under `-eo pipefail`, which is what the workflows declare — the outer
  // shell here is irrelevant to the pipe semantics under test.
  return execSync(`bash --noprofile --norc -eo pipefail -c "${cmd}"`, {
    stdio: 'pipe',
    timeout: 10_000,
  });
}

describe('pipefail shell wrapper (AUDIT-27 CI-04)', () => {
  it('fails the pipeline when the left command exits nonzero', () => {
    let exitCode = 0;
    try {
      runWithPipefailFlags('false | tee /tmp/oz-pipefail-1.log');
    } catch (err) {
      exitCode = err.status ?? 1;
    }
    assert.notStrictEqual(exitCode, 0, 'pipefail must propagate `false | tee` as a failure');
  });

  it('still succeeds when the left command exits zero', () => {
    runWithPipefailFlags('true | tee /tmp/oz-pipefail-2.log');
  });

  it('declares the pipefail shell on every tee-wrapped step in ci.yml', () => {
    const ci = readFileSync(resolve(ROOT, '.github/workflows/ci.yml'), 'utf8');
    const steps = splitSteps(ci);
    const teeSteps = steps.filter((s) => s.includes('| tee '));
    assert.ok(teeSteps.length >= 3, 'ci.yml should have at least 3 tee-wrapped steps');
    for (const step of teeSteps) {
      assert.ok(
        step.includes('shell: bash --noprofile --norc -eo pipefail {0}'),
        `every tee-wrapped step must declare the pipefail shell:\n${step.slice(0, 400)}`,
      );
    }
  });

  it('declares the pipefail shell on every tee-wrapped step in nightly.yml', () => {
    const nightly = readFileSync(resolve(ROOT, '.github/workflows/nightly.yml'), 'utf8');
    const steps = splitSteps(nightly);
    const teeSteps = steps.filter((s) => s.includes('| tee '));
    assert.ok(teeSteps.length >= 2, 'nightly.yml should have at least 2 tee-wrapped steps');
    for (const step of teeSteps) {
      assert.ok(
        step.includes('shell: bash --noprofile --norc -eo pipefail {0}'),
        `every tee-wrapped step must declare the pipefail shell:\n${step.slice(0, 400)}`,
      );
    }
  });

  /** Split a workflow file into individual step blocks (starts at `- name:` or `- uses:`). */
  function splitSteps(workflow) {
    return workflow.split('\n').reduce((acc, line, i, arr) => {
      const isStepStart = /^\s+- (name|uses):/.test(line);
      if (isStepStart && acc.length > 0) acc[acc.length - 1] += '\n';
      if (isStepStart) acc.push(line);
      else if (acc.length > 0) acc[acc.length - 1] += '\n' + line;
      return acc;
    }, []);
  }

  it('runs the report-flaky.sh script under set -euo pipefail', () => {
    const script = readFileSync(resolve(ROOT, 'scripts/report-flaky.sh'), 'utf8');
    assert.ok(script.includes('set -euo pipefail'), 'report-flaky.sh must set -euo pipefail');
  });
});
