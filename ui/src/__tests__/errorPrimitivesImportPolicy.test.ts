// ── Error-primitive import-policy test (ERR-03) ──────────────────
//
// The audit found two ErrorState implementations (ui/src/components and
// ui/src/frontend/shared) that could drift in behavior and styling. The
// fix consolidates the canonical implementation in @/components and turns
// the shared path into a thin re-export.
//
// This test pins that consolidation: it fails at test time if anyone
// reimplements the component in the shared path. The shared files must
// contain ONLY re-export statements (no `function X`/`export function X`).

import { describe, it, expect } from 'vitest';
import fs from 'fs';
import path from 'path';

const SHARED_DIR = path.resolve(__dirname, '../frontend/shared');

const PRIMITIVES = [
  { file: 'ErrorState.tsx', canonical: '@/components/ErrorState' },
  { file: 'EmptyState.tsx', canonical: '@/components/EmptyState' },
  { file: 'Spinner.tsx', canonical: '@/components/Spinner' },
];

function readShared(file: string): string {
  return fs.readFileSync(path.join(SHARED_DIR, file), 'utf-8');
}

describe('error-primitive import policy (ERR-03)', () => {
  it.each(PRIMITIVES.map((p) => [p.file, p.canonical] as const))(
    '%s is a thin re-export of the canonical %s',
    (_file, canonical) => {
      const source = readShared(_file);
      // Must re-export the canonical implementation.
      expect(source).toContain(`export { ${path.basename(_file, '.tsx')} } from '${canonical}'`);
      expect(source).toContain(`from '${canonical}'`);
      // Must NOT contain an implementation (no component function bodies).
      expect(source).not.toMatch(/export function/);
      expect(source).not.toMatch(/function \w+\(/);
      expect(source).not.toMatch(/return \(/);
      // Must not import React or JSX helpers that an implementation would need.
      expect(source).not.toContain("from 'react'");
    },
  );

  it('shared index still re-exports the primitives', () => {
    const index = fs.readFileSync(path.join(SHARED_DIR, 'index.ts'), 'utf-8');
    for (const { file } of PRIMITIVES) {
      const name = path.basename(file, '.tsx');
      expect(index).toContain(`export { ${name} } from './${name}'`);
    }
  });
});
