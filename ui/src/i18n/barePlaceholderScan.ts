// ── Bare Fluent placeholder scanner ───────────────────────────────
//
// Fluent parses `{ created }` — a placeable whose content is a bare
// identifier with no `$` (variable) or `-` (term) prefix — as a
// *message reference*. When no message of that name exists, the real
// runtime renders the literal `{created}` text and records an error.
// Rounds 150–152 shipped exactly that defect in the Apply chip and
// discard dialog; mocked-Fluent tests interpolate by hand and could
// never see it. This module is the permanent guard: a pure per-file
// scan plus a repo-wide scan over every locale bundle, wired into the
// i18n lint gate so a bare placeholder can never ship again.

export interface BarePlaceholderHit {
  file: string;
  line: number;
  ident: string;
}

/** A placeable holding a bare identifier (no `$` / `-` prefix). */
const BARE_PLACEHOLDER = /\{\s*([a-zA-Z][\w-]*)\s*\}/g;

/** Message ids (and the id prefix of each line) in an FTL source. */
function messageIds(source: string): Set<string> {
  return new Set([...source.matchAll(/^([\w-]+)\s*=/gm)].map((m) => m[1]!));
}

/**
 * Find bare-`{}` placeholders in one FTL source. A placeholder whose
 * identifier matches a message defined in the same file is a
 * legitimate message reference and is ignored; anything else is the
 * round-150 defect (renders literally in the real runtime).
 */
export function findBarePlaceholders(
  source: string,
): Array<{ line: number; ident: string }> {
  const ids = messageIds(source);
  const hits: Array<{ line: number; ident: string }> = [];
  for (const match of source.matchAll(BARE_PLACEHOLDER)) {
    const ident = match[1]!;
    if (ids.has(ident)) continue;
    hits.push({ line: source.slice(0, match.index).split('\n').length, ident });
  }
  return hits;
}

/**
 * Scan every locale bundle in `src/locales` (both `.ftl` and
 * `.id.ftl`) for bare placeholders. Returns one hit per occurrence,
 * naming the file and line — the caller's `expect(...).toEqual([])`
 * turns a regression into a readable report.
 */
export function scanLocaleFiles(): BarePlaceholderHit[] {
  const modules = import.meta.glob('../locales/*.ftl', {
    query: '?raw',
    import: 'default',
    eager: true,
  });
  const hits: BarePlaceholderHit[] = [];
  for (const [path, source] of Object.entries(modules)) {
    for (const hit of findBarePlaceholders(source as string)) {
      hits.push({ file: path, ...hit });
    }
  }
  return hits;
}

// ── Localized-vars cross-check (round 164) ───────────────────────
//
// Bundle parity counts keys; it does NOT check that a `<Localized
// id="..." vars={{ ... }}>` site provides exactly the variables the
// FTL message declares. A site missing a `$var` renders the raw id (or
// a partial message) in the real runtime — the same invisible-to-
// mocked-Fluent defect family as the bare placeholders. This scan
// extracts each message's declared `$vars` from the en bundles (via the
// REAL `@fluent/bundle` parser — value and per-attribute separately)
// and each statically-readable `<Localized>` site's `vars` keys, and
// reports any mismatch. Runs in the same i18n gate.

import { FluentResource } from '@fluent/bundle';

export interface LocalizedSite {
  /** The FTL message id (string-literal `id` only). */
  id: string;
  /** Statically-known `vars` object keys; `null` when the `vars`
   *  expression is not a readable object literal (skipped — the
   *  programmatic case, like the parity gate's programmatic ids). */
  varsKeys: string[] | null;
  /** Statically-known `attrs` keys (localized attributes); `null` when
   *  the `attrs` expression is not a readable object literal. */
  attrsKeys: string[] | null;
  line: number;
}

export interface LocalizedVarsHit {
  file: string;
  line: number;
  id: string;
  /** Message `$vars` the site does not provide — renders literally. */
  missing: string[];
  /** Site keys the message does not declare — drift, not breakage. */
  extra: string[];
}

/** A message's variable contract: what its VALUE needs, and what each
 *  localizable ATTRIBUTE needs (a site only pays the attributes it
 *  localizes via `attrs`). */
export interface MessageVarContract {
  value: string[];
  attributes: Map<string, string[]>;
}

/** Every `{ type: 'var' }` name reachable in a Fluent pattern (including
 *  inside selectors and `-term($arg)` call arguments). */
function patternVars(pattern: unknown): string[] {
  const vars = new Set<string>();
  const walk = (node: unknown): void => {
    if (node === null || node === undefined) return;
    if (typeof node === 'string') return;
    if (typeof node !== 'object') return;
    if (Array.isArray(node)) {
      for (const item of node) walk(item);
      return;
    }
    const obj = node as { type?: string; name?: unknown };
    if (obj.type === 'var' && typeof obj.name === 'string') vars.add(obj.name);
    for (const value of Object.values(node)) walk(value);
  };
  walk(pattern);
  return [...vars].sort();
}

/**
 * Every message id → its variable contract (value vars + per-attribute
 * vars), parsed with the REAL `@fluent/bundle` grammar. Using the real
 * parser means the value/attribute split is exactly what the runtime
 * sees — no hand-rolled grammar to drift. Terms are included (their
 * ids start with `-`); no `<Localized>` site references a term, so they
 * never collide.
 */
export function messageDeclaredVars(source: string): Map<string, MessageVarContract> {
  const resource = new FluentResource(source);
  const map = new Map<string, MessageVarContract>();
  for (const entry of resource.body) {
    const id = entry?.id;
    if (typeof id !== 'string') continue; // Junk entries
    const attributes = new Map<string, string[]>();
    for (const [name, pattern] of Object.entries(entry.attributes ?? {})) {
      attributes.set(name, patternVars(pattern));
    }
    map.set(id, { value: patternVars(entry.value), attributes });
  }
  return map;
}

/** The opening `<Localized ...>` tag: from the tag start to its closing
 *  `>` at zero brace depth outside quotes. A `>` inside a prop's nested
 *  JSX truncates the window early — the site is then mis-parsed and its
 *  later `vars` prop missed (a possible false-positive on var-bearing
 *  messages; none exist in the current tree). */
function scanOpeningTagEnd(source: string, start: number): number {
  let braceDepth = 0;
  let quote: string | null = null;
  for (let i = start; i < source.length; i += 1) {
    const ch = source[i]!;
    if (quote) {
      if (ch === quote) quote = null;
      continue;
    }
    if (ch === '"' || ch === "'") {
      quote = ch;
      continue;
    }
    if (ch === '{') braceDepth += 1;
    else if (ch === '}') braceDepth -= 1;
    else if (ch === '>' && braceDepth === 0) return i;
  }
  return -1;
}

/** Top-level keys of a JS object literal `{ ... }` (explicit, quoted, or
 *  shorthand). The object's own braces delimit the top level; inner
 *  braces (nested objects/arrays) are tracked by depth. */
function objectLiteralKeys(literal: string): string[] {
  const keys: string[] = [];
  let depth = 0;
  let quote: string | null = null;
  let current = '';
  const flush = () => {
    const entry = current.trim();
    current = '';
    if (!entry || entry.startsWith('...')) return; // spread — not a key
    const colon = entry.indexOf(':');
    if (colon === -1) {
      keys.push(entry); // shorthand `{ count }`
      return;
    }
    keys.push(entry.slice(0, colon).trim().replace(/^['"]|['"]$/g, ''));
  };
  const inner = literal.slice(1, -1);
  for (const ch of `${inner},`) {
    if (quote) {
      current += ch;
      if (ch === quote) quote = null;
      continue;
    }
    if (ch === '"' || ch === "'") {
      quote = ch;
      current += ch;
      continue;
    }
    if (ch === '{' || ch === '[' || ch === '(') {
      depth += 1;
      current += ch;
      continue;
    }
    if (ch === '}' || ch === ']' || ch === ')') {
      depth -= 1;
      current += ch;
      continue;
    }
    if (ch === ',' && depth === 0) {
      flush();
      continue;
    }
    current += ch;
  }
  flush();
  return [...new Set(keys)];
}

/**
 * The statically-readable keys of a `propName={{ ... }}` object-literal
 * prop, or `null` when the prop is absent from the tag or its value is
 * not a readable literal (a non-literal expression is skipped, like the
 * parity gate's programmatic ids).
 */
function propObjectKeys(tag: string, propName: string): string[] | null {
  const propMatch = new RegExp(`\\b${propName}\\s*=\\s*\\{`).exec(tag);
  if (!propMatch) return null; // absent — distinguishable from unresolvable
  const literalStart = propMatch.index + propMatch[0].length - 1;
  let depth = 0;
  let quote: string | null = null;
  let exprEnd = -1;
  for (let i = literalStart; i < tag.length; i += 1) {
    const ch = tag[i]!;
    if (quote) {
      if (ch === quote) quote = null;
      continue;
    }
    if (ch === '"' || ch === "'") {
      quote = ch;
      continue;
    }
    if (ch === '{') depth += 1;
    else if (ch === '}') {
      depth -= 1;
      if (depth === 0) {
        exprEnd = i;
        break;
      }
    }
  }
  if (exprEnd === -1) return null;
  const expr = tag.slice(literalStart, exprEnd + 1);
  // A literal object is `{{ ... }}` (JSX braces + object braces); a bare
  // `{...}` is an expression — unresolvable statically.
  if (!expr.startsWith('{{')) return null;
  return objectLiteralKeys(expr.slice(1, -1));
}

/**
 * Find every `<Localized>` opening tag with a string-literal `id` and
 * its statically-readable `vars`/`attrs` keys. Sites without a `vars`
 * prop get `varsKeys: []`; sites whose `vars` is a non-literal
 * expression get `varsKeys: null` (skipped). `attrsKeys` is `null` when
 * the site has no statically-readable `attrs` (then only the message's
 * VALUE vars are required of it).
 */
export function findLocalizedSites(source: string): LocalizedSite[] {
  const sites: LocalizedSite[] = [];
  let idx = 0;
  while ((idx = source.indexOf('<Localized', idx)) !== -1) {
    const line = source.slice(0, idx).split('\n').length;
    const tagEnd = scanOpeningTagEnd(source, idx);
    if (tagEnd === -1) break;
    const tag = source.slice(idx, tagEnd);
    const idMatch = /\bid\s*=\s*["']([^"']+)["']/.exec(tag);
    if (idMatch) {
      const hasVars = /\bvars\s*=\s*\{/.test(tag);
      const varsKeys = hasVars ? propObjectKeys(tag, 'vars') : [];
      const attrsKeys = /\battrs\s*=\s*\{/.test(tag) ? propObjectKeys(tag, 'attrs') : [];
      sites.push({ id: idMatch[1]!, varsKeys, attrsKeys, line });
    }
    idx = tagEnd + 1;
  }
  return sites;
}

/**
 * The scan's hit decision, pure: a site's `varsKeys` must exactly match
 * the message's declared `$vars`. Required vars are the message VALUE's
 * plus each localized ATTRIBUTE's — but only when the site actually
 * localizes that attribute (a site paying only the value must not be
 * charged the attribute's vars). `attrsKeys` is `null` when the site's
 * attrs expression is unresolvable (then only value vars are required).
 * Returns the mismatch or `null` when the site is exact.
 */
export function varsMismatch(
  varsKeys: string[],
  contract: MessageVarContract,
  attrsKeys: string[] | null,
): { missing: string[]; extra: string[] } | null {
  const required = new Set(contract.value);
  if (attrsKeys !== null) {
    for (const attr of attrsKeys) {
      for (const v of contract.attributes.get(attr) ?? []) required.add(v);
    }
  }
  const missing = [...required].filter((v) => !varsKeys.includes(v));
  const extra = varsKeys.filter((v) => !required.has(v));
  if (missing.length === 0 && extra.length === 0) return null;
  return { missing, extra };
}

/**
 * Repo-wide cross-check: every `<Localized>` site's statically-known
 * `vars` keys must exactly match the en-bundle message's declared
 * `$vars`. Missing variables render the raw id; extra keys are drift.
 * Sites with unresolvable `vars` expressions and ids absent from the
 * en bundles are skipped (the latter is the parity gate's job).
 */
export function scanLocalizedVars(): LocalizedVarsHit[] {
  // EN bundles only — the canonical variable contract. The `*.id.ftl`
  // translations may legitimately drop a variable (a shorter translation),
  // so they must not overwrite the en declaration.
  const ftlModules = import.meta.glob(['../locales/*.ftl', '!../locales/*.id.ftl'], {
    query: '?raw',
    import: 'default',
    eager: true,
  });
  const declared = new Map<string, MessageVarContract>();
  for (const source of Object.values(ftlModules)) {
    for (const [id, contract] of messageDeclaredVars(source as string)) declared.set(id, contract);
  }

  const tsxModules = import.meta.glob(['../**/*.tsx', '!../**/__tests__/**'], {
    query: '?raw',
    import: 'default',
    eager: true,
  });
  const hits: LocalizedVarsHit[] = [];
  for (const [path, source] of Object.entries(tsxModules)) {
    for (const site of findLocalizedSites(source as string)) {
      const contract = declared.get(site.id);
      if (contract === undefined) continue; // missing key — the parity gate owns that
      if (site.varsKeys === null) continue; // unresolvable vars expression — documented
      const mismatch = varsMismatch(site.varsKeys, contract, site.attrsKeys);
      if (mismatch !== null) {
        hits.push({ file: path, line: site.line, id: site.id, ...mismatch });
      }
    }
  }
  return hits;
}

// ── Translation var drift (round 165) ────────────────────────────
//
// The en-side gate (above) aligns every `<Localized>` site to the en
// contract, so a site can only provide the vars the en message
// declares. An id translation referencing any OTHER variable name
// therefore renders a literal `{$var}` placeholder for Indonesian
// users. This scan checks the SUBSET direction: a translation DROPPING
// a var is safe in Fluent (unused vars are ignored) — only drift (a
// var the en counterpart never declares) is a defect. No skip list is
// needed: legitimate omissions are safe by construction.

export interface TranslationVarDriftHit {
  file: string;
  line: number;
  id: string;
  /** `value` or the attribute name carrying the drifted var(s). */
  attr: string;
  /** The var(s) the id message references that en never declares. */
  vars: string[];
}

/**
 * Pure drift decision: the id contract's vars must be a SUBSET of the
 * en counterpart's, compared per value and per attribute (attributes
 * only when present in BOTH — an id-only attribute is never localized,
 * an en-only attribute is a separate omission defect class). Returns
 * one entry per drifted value/attribute with the offending var names.
 */
export function translationVarDrift(
  idContract: MessageVarContract,
  enContract: MessageVarContract,
): Array<{ attr: string; vars: string[] }> {
  const drift: Array<{ attr: string; vars: string[] }> = [];
  const enValue = new Set(enContract.value);
  const idValueDrift = idContract.value.filter((v) => !enValue.has(v));
  if (idValueDrift.length > 0) drift.push({ attr: 'value', vars: idValueDrift });
  for (const [attr, idVars] of idContract.attributes) {
    const enVars = enContract.attributes.get(attr);
    if (enVars === undefined) continue; // id-only attribute — never localized
    const enAttr = new Set(enVars);
    const drifted = idVars.filter((v) => !enAttr.has(v));
    if (drifted.length > 0) drift.push({ attr, vars: drifted });
  }
  return drift;
}

/**
 * Repo-wide drift scan: every message in every `*.id.ftl` bundle that
 * also exists in the en bundles must reference only vars its en
 * counterpart declares. Missing id keys, en-only keys, and id-only
 * attributes are the parity gate's / a separate defect class's job.
 */
export function scanTranslationVars(): TranslationVarDriftHit[] {
  const ftlModules = import.meta.glob(['../locales/*.ftl', '!../locales/*.id.ftl'], {
    query: '?raw',
    import: 'default',
    eager: true,
  });
  const enDeclared = new Map<string, MessageVarContract>();
  for (const source of Object.values(ftlModules)) {
    for (const [id, contract] of messageDeclaredVars(source as string)) {
      enDeclared.set(id, contract);
    }
  }

  const idModules = import.meta.glob('../locales/*.id.ftl', {
    query: '?raw',
    import: 'default',
    eager: true,
  });
  const hits: TranslationVarDriftHit[] = [];
  for (const [path, source] of Object.entries(idModules)) {
    const text = source as string;
    for (const [id, idContract] of messageDeclaredVars(text)) {
      const enContract = enDeclared.get(id);
      if (enContract === undefined) continue; // id-only key — the parity gate owns that
      const drift = translationVarDrift(idContract, enContract);
      for (const entry of drift) {
        const line = text.slice(0, text.indexOf(`${id} =`)).split('\n').length;
        hits.push({ file: path, line, id, ...entry });
      }
    }
  }
  return hits;
}
