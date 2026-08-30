// @vitest-environment jsdom
import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

/**
 * Tests for Footer.astro link integrity.
 *
 * Footer renders 4 business vertical links and 2 legal links, all driven
 * by `getRelativeLocaleUrl`. A broken href or missing i18n key sends users
 * to 404s on revenue-critical pages.
 *
 * Strategy: read the raw .astro source and verify the link structure is
 * correct and all target pages exist on disk.
 */

const FOOTER_SRC = readFileSync(
  join(import.meta.dirname, '..', '..', 'components', 'Footer.astro'),
  'utf-8',
);

// ─── Link structure tests ────────────────────────────────────────────

describe('Footer link structure', () => {
  it('defines all 4 vertical keys in the source', () => {
    const verticalKeys = ['kafe', 'minimarket', 'warung', 'restoran'];
    for (const key of verticalKeys) {
      expect(FOOTER_SRC).toContain(`'${key}'`);
    }
  });

  it('maps vertical keys to untuk-* hrefs via template literal', () => {
    // The vertical links are generated dynamically: `untuk-${key}`
    expect(FOOTER_SRC).toContain('`untuk-${key}`');
  });

  it('renders 2 legal links (privacy and terms)', () => {
    expect(FOOTER_SRC).toContain("'legal/privacy'");
    expect(FOOTER_SRC).toContain("'legal/terms'");
  });

  it('uses getRelativeLocaleUrl for navigation links', () => {
    // Vertical links use 1 getRelativeLocaleUrl inside .map(), legal has 2
    const matches = FOOTER_SRC.match(/getRelativeLocaleUrl\(/g);
    expect(matches).toHaveLength(3);
  });

  it('has aria-label on the business verticals nav', () => {
    expect(FOOTER_SRC).toContain("aria-label={t(locale, 'footer.business')}");
  });

  it('has aria-label on the legal nav', () => {
    expect(FOOTER_SRC).toContain("aria-label={t(locale, 'footer.legal')}");
  });

  it('has footer-link class on navigation links', () => {
    // 1 template for verticals + 2 static for legal = 3 occurrences
    const footerLinkMatches = FOOTER_SRC.match(/class="footer-link/g);
    expect(footerLinkMatches).toHaveLength(3);
  });
});

// ─── Target page existence tests ─────────────────────────────────────

describe('Footer link targets exist', () => {
  const pagesDir = join(import.meta.dirname, '..', '..', 'pages', '[locale]');

  it('untuk-kafe page exists', () => {
    expect(() => readFileSync(join(pagesDir, 'untuk-kafe.astro'))).not.toThrow();
  });

  it('untuk-minimarket page exists', () => {
    expect(() => readFileSync(join(pagesDir, 'untuk-minimarket.astro'))).not.toThrow();
  });

  it('untuk-warung page exists', () => {
    expect(() => readFileSync(join(pagesDir, 'untuk-warung.astro'))).not.toThrow();
  });

  it('untuk-restoran page exists', () => {
    expect(() => readFileSync(join(pagesDir, 'untuk-restoran.astro'))).not.toThrow();
  });

  it('legal/privacy page exists', () => {
    expect(() => readFileSync(join(pagesDir, 'legal', 'privacy.astro'))).not.toThrow();
  });

  it('legal/terms page exists', () => {
    expect(() => readFileSync(join(pagesDir, 'legal', 'terms.astro'))).not.toThrow();
  });
});

// ─── i18n key tests ──────────────────────────────────────────────────

describe('Footer i18n keys', () => {
  const enJson = JSON.parse(
    readFileSync(join(import.meta.dirname, '..', '..', 'i18n', 'en.json'), 'utf-8'),
  );
  const idJson = JSON.parse(
    readFileSync(join(import.meta.dirname, '..', '..', 'i18n', 'id.json'), 'utf-8'),
  );

  it('en footer has all required keys', () => {
    expect(enJson.footer).toBeDefined();
    expect(enJson.footer.rights).toBeTruthy();
    expect(enJson.footer.privacy).toBeTruthy();
    expect(enJson.footer.terms).toBeTruthy();
    expect(enJson.footer.legal).toBeTruthy();
    expect(enJson.footer.business).toBeTruthy();
  });

  it('id footer has all required keys', () => {
    expect(idJson.footer).toBeDefined();
    expect(idJson.footer.rights).toBeTruthy();
    expect(idJson.footer.privacy).toBeTruthy();
    expect(idJson.footer.terms).toBeTruthy();
    expect(idJson.footer.legal).toBeTruthy();
    expect(idJson.footer.business).toBeTruthy();
  });

  it('en footer vertical labels are non-empty', () => {
    const verticalKeys = ['kafe', 'minimarket', 'warung', 'restoran'] as const;
    for (const key of verticalKeys) {
      expect(enJson.vertical[key].label).toBeTruthy();
    }
  });

  it('id footer vertical labels are non-empty', () => {
    const verticalKeys = ['kafe', 'minimarket', 'warung', 'restoran'] as const;
    for (const key of verticalKeys) {
      expect(idJson.vertical[key].label).toBeTruthy();
    }
  });

  it('en and id have the same footer keys', () => {
    const enKeys = Object.keys(enJson.footer).sort();
    const idKeys = Object.keys(idJson.footer).sort();
    expect(idKeys).toEqual(enKeys);
  });

  it('en and id vertical labels differ (translated)', () => {
    const verticalKeys = ['kafe', 'minimarket', 'warung', 'restoran'] as const;
    const differences = verticalKeys.filter(
      (key) => enJson.vertical[key].label !== idJson.vertical[key].label,
    );
    expect(differences.length).toBeGreaterThanOrEqual(1);
  });
});

// ─── Copyright year test ─────────────────────────────────────────────

describe('Footer copyright', () => {
  it('uses dynamic year via Date constructor', () => {
    expect(FOOTER_SRC).toContain('new Date().getFullYear()');
  });

  it('has the OZ-POS brand name', () => {
    expect(FOOTER_SRC).toContain('OZ-POS');
  });

  it('has Discord social link', () => {
    expect(FOOTER_SRC).toContain('discord.gg');
    expect(FOOTER_SRC).toContain('aria-label="Discord"');
  });
});
