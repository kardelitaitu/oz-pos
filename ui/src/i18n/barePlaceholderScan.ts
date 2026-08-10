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
