// ── i18n stray-attribute-syntax guard ────────────────────────────────
//
// Fluent attributes are declared on INDENTED lines:
//
//     key =
//         .aria-label = Label
//
// A message written on a SINGLE line as `key = .aria-label = Label` is
// NOT an attribute — Fluent parses it as a literal text VALUE equal to
// `.aria-label = Label`. Since these `-aria` keys are consumed with
// `l10n.getString(id)` (which returns the message VALUE, never its
// attributes), the rendered `aria-label` was literally the string
// `.aria-label = Label` — the broken prefix leaked into the UI and
// screen readers.
//
// This test parses the real production bundles and fails if any message
// VALUE still starts with a `.`-attribute marker. It is the permanent
// regression guard against re-introducing the single-line form.
import { describe, it, expect } from 'vitest';
import { FluentBundle, FluentResource } from '@fluent/bundle';
import reportsEn from '@/locales/reports.ftl?raw';
import reportsId from '@/locales/reports.id.ftl?raw';

/**
 * Return every `{ key, value }` whose formatted message VALUE still
 * begins with a stray attribute marker (`.<name> = …`), i.e. keys that
 * were written with the invalid single-line attribute syntax.
 */
function scanStrayAttributes(ftl: string): Array<{ key: string; value: string }> {
  const bundle = new FluentBundle('en', { useIsolating: false });
  bundle.addResource(new FluentResource(ftl));
  const out: Array<{ key: string; value: string }> = [];
  for (const line of ftl.split('\n')) {
    const m = line.match(/^([a-zA-Z0-9_-]+)\s*=/);
    if (!m || !m[1]) continue;
    const msg = bundle.getMessage(m[1]);
    if (!msg?.value) continue;
    const value = bundle.formatPattern(msg.value, null, []);
    if (/^\.\w+\s*=/.test(value)) out.push({ key: m[1], value });
  }
  return out;
}

describe('i18n stray attribute syntax (reports bundles)', () => {
  it('reports.ftl has no message value that starts with a `.attr =` marker', () => {
    expect(scanStrayAttributes(reportsEn)).toEqual([]);
  });

  it('reports.id.ftl has no message value that starts with a `.attr =` marker', () => {
    expect(scanStrayAttributes(reportsId)).toEqual([]);
  });
});
