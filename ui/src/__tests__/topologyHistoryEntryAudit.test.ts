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
 * needed. It scans ALL of ui/src for undo/redo-stack entry-creation sites
 * (any `set<…>((prev) => …)` updater whose setter names History/Redo/Undo
 * and whose body spreads `prev` into a new array — append or prepend) and
 * asserts every one either routes through historyEntry or is declared in
 * DOCUMENTED_EXCEPTIONS below. The topology editor's four sanitized sites
 * are the only non-exempt producers; a NEW raw push anywhere in the tree —
 * especially a future graph editor's own stack — fails until it is routed
 * through historyEntry or explicitly declared.
 */

import { describe, expect, it } from 'vitest';
import { readdirSync, readFileSync } from 'fs';
import { join, relative, resolve } from 'path';

/* ── Paths ───────────────────────────────────────────────────── */

const UI_SRC = resolve(__dirname, '..');
const EDITOR_SRC = resolve(UI_SRC, 'features/stores/NodeTopologyEditor.tsx');

/* ── Drift-guard baseline ─────────────────────────────────────── */
// Every undo/redo entry-creation site must route through historyEntry().
//
// Baseline: 4 entry-creating updaters in the topology editor.
//   1. pushHistory          — setHistory block body (every mutation path)
//   2. commitDuplicateDrag  — setHistory block body (the filtered entry)
//   3. popUndo              — setRedo push (current state → redo branch)
//   4. popRedo              — setHistory push (current state → history)
//
// When a new history-entry producer is added, route it through
// historyEntry() AND increment this count. When one is removed or
// refactored into a helper, update the count and re-check the list.
const EXPECTED_ENTRY_CREATORS = 4;

/* ── Documented exceptions (whole-tree scan) ──────────────────── */
// Undo-stack entry creators that deliberately do NOT go through
// historyEntry(). historyEntry sanitizes graph wires against the entry's
// node set — an invariant that only exists where entries carry
// cross-references. A flat item-undo stack has no such invariant, so it is
// exempt by design; any OTHER creator must use historyEntry.
const DOCUMENTED_EXCEPTIONS: { file: string; setter: string; reason: string }[] = [
  {
    file: 'features/retail/RetailPosScreen.tsx',
    setter: 'setUndoStack',
    reason: 'cart-item undo (removed-line LIFO) — entries carry no cross-references, so the graph wire/node invariant has no analogue',
  },
];

/* ── Scanner ──────────────────────────────────────────────────── */

/** Remove comments and string literals so structural scanning cannot be
 *  unbalanced by prose parens inside comments or string content. Also
 *  returns origIndexAt[i] — the ORIGINAL source index of the i-th emitted
 *  char — so scan positions can be mapped back for accurate line numbers. */
function stripCommentsAndStrings(src: string): { stripped: string; origIndexAt: number[] } {
  let out = '';
  const origIndexAt: number[] = [];
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
      origIndexAt.push(i);
      i += 1;
    }
  }
  return { stripped: out, origIndexAt };
}

interface UpdaterSite {
  setter: string;
  body: string;
  index: number;
}

/** Extract every `set<…>((prev) => …)` updater body via balanced
 *  delimiters (on comment/string-stripped source). */
function extractUpdaterBodies(src: string): UpdaterSite[] {
  const out: UpdaterSite[] = [];
  const re = /(set[A-Za-z0-9_$]*)\(\(prev\)\s*=>\s*/g;
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

/** An undo/redo-stack entry creator: a History/Redo/Undo-named setter whose
 *  updater pushes by spreading `prev` into a new array (append or prepend).
 *  site.index is an ORIGINAL-source index (mapped through origIndexAt). */
function historyEntryCreators(src: string): UpdaterSite[] {
  const { stripped, origIndexAt } = stripCommentsAndStrings(src);
  return extractUpdaterBodies(stripped)
    .filter((u) => /History|Redo|Undo/i.test(u.setter) && u.body.includes('...prev'))
    .map((u) => ({ ...u, index: origIndexAt[u.index] ?? u.index }));
}

/** Recursively collect production .ts/.tsx sources under `dir` (tests and
 *  build artifacts excluded — the audit targets application code). */
function collectSourceFiles(dir: string): string[] {
  const out: string[] = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      if (entry.name === 'node_modules' || entry.name === '__tests__' || entry.name.startsWith('.')) continue;
      out.push(...collectSourceFiles(full));
    } else if (
      (entry.name.endsWith('.ts') || entry.name.endsWith('.tsx')) &&
      !entry.name.endsWith('.d.ts')
    ) {
      out.push(full);
    }
  }
  return out;
}

/* ── Editor-scoped audit (line-precise messages) ───────────────── */

describe('history-entry producer audit', () => {
  const source = readFileSync(EDITOR_SRC, 'utf-8');
  const creating = historyEntryCreators(source);

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

/* ── Whole-tree audit ──────────────────────────────────────────── */

describe('history-entry producer audit — whole ui/src', () => {
  interface TreeSite extends UpdaterSite {
    file: string;
    line: number;
  }

  const treeSites: TreeSite[] = [];
  for (const file of collectSourceFiles(UI_SRC)) {
    const src = readFileSync(file, 'utf-8');
    for (const site of historyEntryCreators(src)) {
      treeSites.push({
        ...site,
        file: relative(UI_SRC, file).replace(/\\/g, '/'),
        line: src.slice(0, site.index).split('\n').length,
      });
    }
  }

  const isExempt = (site: TreeSite) =>
    DOCUMENTED_EXCEPTIONS.some((e) => e.file === site.file && e.setter === site.setter);

  it('every undo/redo entry creator in ui/src uses historyEntry or is a documented exception', () => {
    for (const site of treeSites) {
      const exempt = isExempt(site);
      expect(
        exempt || site.body.includes('historyEntry'),
        `${site.file}:${site.line} — ${site.setter} pushes an undo/redo entry without ` +
          'historyEntry() sanitization. Route it through historyEntry() (topologyHistoryIntegrity) ' +
          'or declare it in DOCUMENTED_EXCEPTIONS with a reason.',
      ).toBe(true);
    }
  });

  it('the only non-exempt entry creators are the topology editor\'s four sanitized sites', () => {
    const nonExempt = treeSites.filter((s) => !isExempt(s));
    const listing = nonExempt.map((s) => `${s.file}:${s.line} ${s.setter}`).join('\n  ');
    expect(
      nonExempt.length,
      `expected the topology editor's ${EXPECTED_ENTRY_CREATORS} sanitized sites as the only ` +
        `non-exempt creators, found ${nonExempt.length}:\n  ${listing}`,
    ).toBe(EXPECTED_ENTRY_CREATORS);
    for (const site of nonExempt) {
      expect(site.file).toBe('features/stores/NodeTopologyEditor.tsx');
      expect(site.body).toContain('historyEntry');
    }
  });
});
