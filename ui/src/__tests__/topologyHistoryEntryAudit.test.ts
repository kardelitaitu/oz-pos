/**
 * History-Entry Producer Audit (coverage-style drift guard)
 *
 * Every setHistory/setRedo updater that PUSHES a new entry onto the undo or
 * redo stack must build it with `historyEntry()` (topologyHistoryIntegrity),
 * which sanitizes the entry's wires against its own node set at push time.
 * A raw `{ nodes, wires }` push is a corruption hole: a dangling wire could
 * enter the stack and depend entirely on the restore-boundary guard.
 *
 * This is a static source audit, same approach as
 * noiseDitherCompliance.test.ts / themeTokenCompliance.test.ts — no browser
 * needed. It scans NodeTopologyEditor.tsx for every `setHistory((prev) =>
 * ...` / `setRedo((prev) => ...` updater, classifies the ones that push
 * (`[...prev, ...]`), and asserts each pushes via historyEntry AND that the
 * count matches the baseline below. Fails when a new entry-creation site is
 * added without historyEntry, or when an existing site stops using it.
 */

import { describe, expect, it } from 'vitest';
import { readFileSync } from 'fs';
import { resolve } from 'path';

/* ── Paths ───────────────────────────────────────────────────── */

const UI_SRC = resolve(__dirname, '..');
const EDITOR_SRC = resolve(UI_SRC, 'features/stores/NodeTopologyEditor.tsx');

/* ── Drift-guard baseline ─────────────────────────────────────── */
// Every undo/redo entry-creation site must route through historyEntry().
//
// Baseline: 4 entry-creating updaters.
//   1. pushHistory          — setHistory block body (every mutation path)
//   2. commitDuplicateDrag  — setHistory block body (the filtered entry)
//   3. popUndo              — setRedo push (current state → redo branch)
//   4. popRedo              — setHistory push (current state → history)
//
// When a new history-entry producer is added, route it through
// historyEntry() AND increment this count. When one is removed or
// refactored into a helper, update the count and re-check the list.
const EXPECTED_ENTRY_CREATORS = 4;

/* ── Scanner ──────────────────────────────────────────────────── */

/** Remove comments and string literals so structural scanning cannot be
 *  unbalanced by prose parens inside comments or string content. */
function stripCommentsAndStrings(src: string): string {
  let out = '';
  let i = 0;
  while (i < src.length) {
    const c = src[i]!;
    const n = src[i + 1];
    if (c === '/' && n === '/') {
      while (i < src.length && src[i] !== '\n') i += 1;
    } else if (c === '/' && n === '*') {
      i += 2;
      while (i < src.length && !(src[i] === '*' && src[i + 1] === '/')) i += 1;
      i += 2;
    } else if (c === '"' || c === "'") {
      const q = c;
      i += 1;
      while (i < src.length) {
        if (src[i] === '\\') i += 2;
        else if (src[i] === q) {
          i += 1;
          break;
        } else i += 1;
      }
    } else if (c === '`') {
      i += 1;
      while (i < src.length) {
        if (src[i] === '\\') i += 2;
        else if (src[i] === '`') {
          i += 1;
          break;
        } else if (src[i] === '$' && src[i + 1] === '{') {
          let d = 1;
          i += 2;
          while (i < src.length && d > 0) {
            if (src[i] === '{') d += 1;
            else if (src[i] === '}') d -= 1;
            i += 1;
          }
        } else i += 1;
      }
    } else {
      out += c;
      i += 1;
    }
  }
  return out;
}

interface UpdaterSite {
  setter: string;
  body: string;
  index: number;
}

/** Extract every `setHistory((prev) => …)` / `setRedo((prev) => …)` updater
 *  body via balanced delimiters (on comment/string-stripped source). */
function extractUpdaterBodies(src: string): UpdaterSite[] {
  const out: UpdaterSite[] = [];
  const re = /(setHistory|setRedo)\(\(prev\)\s*=>\s*/g;
  for (const m of src.matchAll(re)) {
    const start = m.index! + m[0].length;
    let depth = 0;
    let i = start;
    for (; i < src.length; i += 1) {
      const c = src[i]!;
      if (c === '(' || c === '{' || c === '[') depth += 1;
      else if (c === ')' || c === '}' || c === ']') {
        depth -= 1;
        if (depth < 0) break;
      }
    }
    out.push({ setter: m[1]!, body: src.slice(start, i), index: m.index! });
  }
  return out;
}

describe('history-entry producer audit', () => {
  const source = readFileSync(EDITOR_SRC, 'utf-8');
  const stripped = stripCommentsAndStrings(source);
  const creating = extractUpdaterBodies(stripped).filter((u) => u.body.includes('[...prev,'));

  const lineOf = (index: number) => source.slice(0, index).split('\n').length;

  it('every entry-creating setHistory/setRedo updater goes through historyEntry', () => {
    for (const site of creating) {
      expect(
        site.body,
        `${site.setter} entry creation at line ${lineOf(site.index)} must build the entry with ` +
          'historyEntry() (topologyHistoryIntegrity) — a raw { nodes, wires } push is a ' +
          'corruption hole that skips push-time wire sanitization.',
      ).toContain('historyEntry');
    }
  });

  it('the number of entry-creation sites matches the drift-guard baseline', () => {
    const sites = creating.map((u) => `${u.setter} (line ${lineOf(u.index)})`).join('\n  ');
    expect(
      creating.length,
      `expected exactly ${EXPECTED_ENTRY_CREATORS} entry-creation site(s), found ${creating.length}:\n  ` +
        sites +
        '\nEvery new producer must use historyEntry() and the baseline must be updated ' +
        '(see the EXPECTED_ENTRY_CREATORS comment in this test).',
    ).toBe(EXPECTED_ENTRY_CREATORS);
  });
});
