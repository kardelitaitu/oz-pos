import { describe, it, expect } from 'vitest';
import { t, dict } from '../i18n';

/**
 * TDD tests for the /tdd (Test-Driven Development) page.
 * These must FAIL before implementation, then PASS after.
 *
 * The /tdd page explains OZ-POS's TDD workflow to developers and
 * contributors — the 7-phase loop, fast TDD tooling, and per-layer
 * testing conventions.
 */

describe('tdd page — i18n dictionary', () => {
  it('has a tdd section in the English dictionary', () => {
    const d = dict('en') as Record<string, unknown>;
    expect(d).toHaveProperty('tdd');
  });

  it('has a tdd section in the Indonesian dictionary', () => {
    const d = dict('id') as Record<string, unknown>;
    expect(d).toHaveProperty('tdd');
  });

  it('tdd.title resolves to a non-empty string in English', () => {
    const title = t('en', 'tdd.title');
    expect(title).not.toBe('tdd.title');
    expect(title.length).toBeGreaterThan(0);
  });

  it('tdd.subtitle resolves to a non-empty string in English', () => {
    const subtitle = t('en', 'tdd.subtitle');
    expect(subtitle).not.toBe('tdd.subtitle');
    expect(subtitle.length).toBeGreaterThan(0);
  });

  it('tdd.description resolves to a non-empty string in English', () => {
    const desc = t('en', 'tdd.description');
    expect(desc).not.toBe('tdd.description');
    expect(desc.length).toBeGreaterThan(0);
  });

  it('tdd.phases is an array with 7 items (the 7-phase loop)', () => {
    const d = dict('en') as Record<string, unknown>;
    const tdd = d.tdd as Record<string, unknown>;
    expect(Array.isArray(tdd.phases)).toBe(true);
    expect((tdd.phases as unknown[]).length).toBe(7);
  });

  it('each phase has a title and description', () => {
    const d = dict('en') as Record<string, unknown>;
    const tdd = d.tdd as Record<string, unknown>;
    const phases = tdd.phases as { title: string; description: string }[];
    for (const phase of phases) {
      expect(phase.title.length).toBeGreaterThan(0);
      expect(phase.description.length).toBeGreaterThan(0);
    }
  });

  it('tdd.goldenRules is an array with at least 5 items', () => {
    const d = dict('en') as Record<string, unknown>;
    const tdd = d.tdd as Record<string, unknown>;
    expect(Array.isArray(tdd.goldenRules)).toBe(true);
    expect((tdd.goldenRules as unknown[]).length).toBeGreaterThanOrEqual(5);
  });

  it('tdd.tools is an array describing the TDD tooling', () => {
    const d = dict('en') as Record<string, unknown>;
    const tdd = d.tdd as Record<string, unknown>;
    expect(Array.isArray(tdd.tools)).toBe(true);
    expect((tdd.tools as unknown[]).length).toBeGreaterThan(0);
  });

  it('Indonesian translations resolve (not fallback keys)', () => {
    const idTitle = t('id', 'tdd.title');
    expect(idTitle).not.toBe('tdd.title');
    // Title should differ from English (it's translated)
    const enTitle = t('en', 'tdd.title');
    expect(idTitle).not.toBe(enTitle);
  });
});

describe('tdd page — page meta', () => {
  it('pageDesc.tdd exists in the English dictionary for meta description', () => {
    const desc = t('en', 'pageDesc.tdd');
    expect(desc).not.toBe('pageDesc.tdd');
    expect(desc.length).toBeGreaterThan(0);
  });

  it('nav.tdd exists for navigation link label', () => {
    const label = t('en', 'nav.tdd');
    expect(label).not.toBe('nav.tdd');
    expect(label.length).toBeGreaterThan(0);
  });
});

describe('tdd page — en/id parity', () => {
  it('both locales have the same tdd keys', () => {
    const en = dict('en') as Record<string, unknown>;
    const id = dict('id') as Record<string, unknown>;
    const enTdd = en.tdd as Record<string, unknown>;
    const idTdd = id.tdd as Record<string, unknown>;

    const enKeys = Object.keys(enTdd).sort();
    const idKeys = Object.keys(idTdd).sort();
    expect(idKeys).toEqual(enKeys);
  });

  it('both locales have the same number of phases', () => {
    const en = dict('en') as Record<string, unknown>;
    const id = dict('id') as Record<string, unknown>;
    const enPhases = (en.tdd as Record<string, unknown>).phases as unknown[];
    const idPhases = (id.tdd as Record<string, unknown>).phases as unknown[];
    expect(idPhases.length).toBe(enPhases.length);
  });

  it('both locales have the same number of golden rules', () => {
    const en = dict('en') as Record<string, unknown>;
    const id = dict('id') as Record<string, unknown>;
    const enRules = (en.tdd as Record<string, unknown>).goldenRules as unknown[];
    const idRules = (id.tdd as Record<string, unknown>).goldenRules as unknown[];
    expect(idRules.length).toBe(enRules.length);
  });
});
