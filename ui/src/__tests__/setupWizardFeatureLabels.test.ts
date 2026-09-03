// ── Setup wizard feature-label parity ───────────────────────────────
//
// SetupWizard builds each row's label as
// `requiredLocalized(l10n, \`setup-feature-${f.key}-label\`)`. A template
// literal is invisible to scripts/verify-bundle-parity.py, so the only thing
// standing between a missing translation and a user reading the raw message
// id "setup-feature-tax-engine-label" is this test.
//
// Ids are enumerated from the raw .ftl source rather than from the component,
// so adding a feature without its translation fails here. Same ?raw approach
// i18nBundle.test.tsx uses; it avoids reaching into FluentBundle privates.

import { describe, it, expect } from 'vitest';
import { getBundle } from '@/i18n';
import settingsEn from '@/locales/settings.ftl?raw';
import settingsId from '@/locales/settings.id.ftl?raw';

const LABEL_KEY = /^setup-feature-[a-z0-9-]+-label[ \t]*=/gm;

function labelKeys(source: string): string[] {
  return [...source.matchAll(LABEL_KEY)]
    .map((m) => m[0].replace(/[ \t]*=$/, ''))
    .sort();
}

describe('setup-feature-*-label', () => {
  const en = labelKeys(settingsEn);
  const id = labelKeys(settingsId);

  it('declares the same label ids in both source files', () => {
    expect(en).toEqual(id);
  });

  it('enumerates a non-trivial set (guards a vacuous pass)', () => {
    expect(en.length).toBeGreaterThanOrEqual(27);
  });

  for (const [locale, source] of [['en', settingsEn], ['id', settingsId]] as const) {
    it(`every label resolves to real text in the ${locale} bundle`, () => {
      const bundle = getBundle(locale);
      for (const key of labelKeys(source)) {
        expect(bundle.hasMessage(key), `${key} absent from ${locale}`).toBe(true);
        const formatted = bundle.formatPattern(bundle.getMessage(key)!.value!, null);
        expect(formatted, `${key} empty in ${locale}`).not.toBe('');
        expect(formatted, `${key} unresolved in ${locale}`).not.toBe(key);
      }
    });
  }
});
