// ── barePlaceholderScan unit tests ────────────────────────────────
//
// Pins the bare-`{}` placeholder guard (round 156). Fluent treats a
// placeable containing a bare identifier — `{ created }` instead of
// `{ $created }` — as a *message reference* to a message of that name;
// when no such message exists, the real runtime renders the literal
// `{created}` text and records an error. Rounds 150–152 shipped this
// defect in the Apply chip and discard dialog; only a mocked Fluent
// (which interpolates by hand) hid it. This scanner is the permanent
// guard: every locale file must be free of bare placeholders whose
// identifier is not a defined message.

import { describe, expect, it } from 'vitest';
import { findBarePlaceholders, scanLocaleFiles } from '@/i18n/barePlaceholderScan';

describe('findBarePlaceholders', () => {
  it('flags a bare identifier placeholder with no matching message', () => {
    expect(findBarePlaceholders('chip = { created } created')).toEqual([
      { line: 1, ident: 'created' },
    ]);
  });

  it('ignores $-variables', () => {
    expect(findBarePlaceholders('chip = { $created } created')).toEqual([]);
  });

  it('ignores term references', () => {
    expect(findBarePlaceholders('brand = {-oz-brand} is great')).toEqual([]);
  });

  it('ignores a message reference whose message is defined', () => {
    const src = ['retry = Retry', 'cta = { retry } now'].join('\n');
    expect(findBarePlaceholders(src)).toEqual([]);
  });

  it('flags a bare placeholder inside a message attribute', () => {
    const src = ['msg = value', '    .label = { oops }'].join('\n');
    expect(findBarePlaceholders(src)).toEqual([{ line: 2, ident: 'oops' }]);
  });

  it('reports the line number of the offending placeholder', () => {
    const src = ['first = ok', 'second = ok', 'third = { nope }'].join('\n');
    expect(findBarePlaceholders(src)).toEqual([{ line: 3, ident: 'nope' }]);
  });

  it('does not flag braces that are not bare identifiers (quoted strings, selectors)', () => {
    const src = [
      'lit = { "" } literal brace',
      'sel = { $count ->',
      '    [one] One',
      '   *[other] { $count }',
      '}',
    ].join('\n');
    expect(findBarePlaceholders(src)).toEqual([]);
  });
});

describe('scanLocaleFiles (repo integrity)', () => {
  it('finds no bare placeholders in any locale bundle', () => {
    // The round-150/152 defect shipped because no gate formatted the
    // keys through the real runtime. Every .ftl / .id.ftl must be
    // free of bare `{ ident }` placeholders whose ident is not a
    // defined message — or this test names the file and line.
    expect(scanLocaleFiles()).toEqual([]);
  });
});
