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
import {
  findBarePlaceholders,
  scanLocaleFiles,
  messageDeclaredVars,
  findLocalizedSites,
  scanLocalizedVars,
  varsMismatch,
} from '@/i18n/barePlaceholderScan';

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

// ── Localized-vars cross-check (round 164) ────────────────────────

describe('messageDeclaredVars', () => {
  it('collects $vars from the message value', () => {
    expect(messageDeclaredVars('counts = { $onlyInCurrent } and { $other } here')).toEqual(
      new Map([['counts', { value: ['onlyInCurrent', 'other'], attributes: new Map() }]]),
    );
  });

  it('collects $vars per message attribute', () => {
    const src = ['msg = body', '    .title = { $tier } tier'].join('\n');
    expect(messageDeclaredVars(src)).toEqual(
      new Map([['msg', { value: [], attributes: new Map([['title', ['tier']]]) }]]),
    );
  });

  it('keeps multiple attributes separate', () => {
    const src = ['msg = { $body }', '    .title = { $tier }', '    .aria = { $tier } here'].join('\n');
    expect(messageDeclaredVars(src)).toEqual(
      new Map([['msg', { value: ['body'], attributes: new Map([['title', ['tier']], ['aria', ['tier']]]) }]]),
    );
  });

  it('treats member access on a variable as the base name', () => {
    expect(messageDeclaredVars('hi = Hello { $user.name }')).toEqual(
      new Map([['hi', { value: ['user'], attributes: new Map() }]]),
    );
  });

  it('returns an empty contract for a plain message', () => {
    expect(messageDeclaredVars('plain = Just text')).toEqual(
      new Map([['plain', { value: [], attributes: new Map() }]]),
    );
  });

  it('captures vars from term call arguments (the real-parser AST walk sees them)', () => {
    // A `-term($name)` call passes a message-level variable inside parens,
    // not a `{$var}` placeable — a hand-rolled regex would miss it, but
    // the real `@fluent/bundle` AST walk finds the `var` node.
    expect(messageDeclaredVars('x = {-brand($name)} rocks')).toEqual(
      new Map([['x', { value: ['name'], attributes: new Map() }]]),
    );
  });

  it('keeps per-message contracts separate across multiple messages', () => {
    const src = ['a = { $x }', 'b = { $y }'].join('\n');
    expect(messageDeclaredVars(src)).toEqual(
      new Map([
        ['a', { value: ['x'], attributes: new Map() }],
        ['b', { value: ['y'], attributes: new Map() }],
      ]),
    );
  });
});

describe('findLocalizedSites', () => {
  it('reads explicit vars keys', () => {
    const src = '<Localized id="k" vars={{ a: 1, b: 2 }}>x</Localized>';
    expect(findLocalizedSites(src)).toEqual([{ id: 'k', varsKeys: ['a', 'b'], attrsKeys: [], line: 1 }]);
  });

  it('reads shorthand vars keys and quoted keys', () => {
    const src = "<Localized id='k' vars={{ created, 'named': 1 }}>x</Localized>";
    const [site] = findLocalizedSites(src);
    expect(site!.varsKeys!.sort()).toEqual(['created', 'named']);
    expect(site!.attrsKeys).toEqual([]);
  });

  it('ignores nested object values and spreads', () => {
    const src = '<Localized id="k" vars={{ a: { deep: true }, b: 1 }}>x</Localized>';
    expect(findLocalizedSites(src)).toEqual([{ id: 'k', varsKeys: ['a', 'b'], attrsKeys: [], line: 1 }]);
  });

  it('reads the attrs keys the site localizes', () => {
    const src = '<Localized id="k" attrs={{ "aria-label": true }} vars={{ name: 1 }}>x</Localized>';
    expect(findLocalizedSites(src)).toEqual([{ id: 'k', varsKeys: ['name'], attrsKeys: ['aria-label'], line: 1 }]);
  });

  it('reports a site with no vars prop as an empty set', () => {
    expect(findLocalizedSites('<Localized id="k">x</Localized>')).toEqual([
      { id: 'k', varsKeys: [], attrsKeys: [], line: 1 },
    ]);
  });

  it('marks an unresolvable vars expression as null (skipped)', () => {
    expect(findLocalizedSites('<Localized id="k" vars={t(vars)}>x</Localized>')).toEqual([
      { id: 'k', varsKeys: null, attrsKeys: [], line: 1 },
    ]);
  });

  it('reports the site line across multiline props', () => {
    const src = [
      '<div>',
      '  <Localized',
      '    id="k"',
      '    vars={{ count: 1 }}',
      '  >',
      '    text',
      '  </Localized>',
      '</div>',
    ].join('\n');
    expect(findLocalizedSites(src)).toEqual([{ id: 'k', varsKeys: ['count'], attrsKeys: [], line: 2 }]);
  });
});

describe('varsMismatch', () => {
  const contract = {
    value: ['name'],
    attributes: new Map([
      ['aria-label', ['name']],
      ['title', ['tier']],
    ]),
  };

  it('is exact when the site provides exactly the value vars', () => {
    expect(varsMismatch(['name'], contract, [])).toBeNull();
  });

  it('flags a missing value var', () => {
    expect(varsMismatch([], contract, [])).toEqual({ missing: ['name'], extra: [] });
  });

  it('flags an extra (drift) key', () => {
    expect(varsMismatch(['name', 'count'], contract, [])).toEqual({
      missing: [],
      extra: ['count'],
    });
  });

  it('charges an attribute\'s vars only when the site localizes it', () => {
    // Not localizing the title attribute must not require `tier`.
    expect(varsMismatch(['name'], contract, ['title'])).toEqual({
      missing: ['tier'],
      extra: [],
    });
    expect(varsMismatch(['name', 'tier'], contract, ['title'])).toBeNull();
  });

  it('does not charge attribute vars when attrs are unresolvable (null)', () => {
    expect(varsMismatch(['name'], contract, null)).toBeNull();
  });
});

describe('scanLocalizedVars (repo integrity)', () => {
  it('finds no vars mismatches across every Localized site and en bundle', () => {
    // A site whose vars keys don't exactly match the message's declared
    // $vars renders the raw id at runtime — invisible to mocked Fluent
    // and to bundle parity (which counts keys, not variables). This
    // scan runs inside the lint:i18n gate so a mismatch fails closed
    // with the file, line, id, and the missing/extra names.
    expect(scanLocalizedVars()).toEqual([]);
  });
});
