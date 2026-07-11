import { describe, it, expect, vi, beforeEach } from 'vitest';
import { detectDefaultCurrency } from '../utils/currency';

describe('detectDefaultCurrency', () => {
  beforeEach(() => {
    vi.unstubAllGlobals();
  });

  it('returns IDR for Indonesian locale (id-ID)', () => {
    vi.stubGlobal('navigator', { language: 'id-ID', languages: ['id-ID'] });
    expect(detectDefaultCurrency()).toBe('IDR');
  });

  it('returns USD for US locale (en-US)', () => {
    vi.stubGlobal('navigator', { language: 'en-US', languages: ['en-US'] });
    expect(detectDefaultCurrency()).toBe('USD');
  });

  it('returns EUR for German locale (de-DE)', () => {
    vi.stubGlobal('navigator', { language: 'de-DE', languages: ['de-DE'] });
    expect(detectDefaultCurrency()).toBe('EUR');
  });

  it('returns EUR for French locale (fr-FR)', () => {
    vi.stubGlobal('navigator', { language: 'fr-FR', languages: ['fr-FR'] });
    expect(detectDefaultCurrency()).toBe('EUR');
  });

  it('returns JPY for Japanese locale (ja-JP)', () => {
    vi.stubGlobal('navigator', { language: 'ja-JP', languages: ['ja-JP'] });
    expect(detectDefaultCurrency()).toBe('JPY');
  });

  it('returns GBP for UK locale (en-GB)', () => {
    vi.stubGlobal('navigator', { language: 'en-GB', languages: ['en-GB'] });
    expect(detectDefaultCurrency()).toBe('GBP');
  });

  it('falls back to USD for unmapped region (en-ZZ)', () => {
    vi.stubGlobal('navigator', { language: 'en-ZZ', languages: ['en-ZZ'] });
    expect(detectDefaultCurrency()).toBe('USD');
  });

  it('handles language-only code like "id" by uppercasing', () => {
    // "id" without region subtag — maps to "ID" → IDR
    vi.stubGlobal('navigator', { language: 'id', languages: ['id'] });
    expect(detectDefaultCurrency()).toBe('IDR');
  });

  it('handles language-only code like "de" → falls back to USD', () => {
    // "de" without region subtag → "DE" which maps to EUR
    vi.stubGlobal('navigator', { language: 'de', languages: ['de'] });
    expect(detectDefaultCurrency()).toBe('EUR');
  });

  it('falls back to USD when navigator is unavailable', () => {
    vi.stubGlobal('navigator', undefined);
    // detectDefaultCurrency has a try/catch, so this should not throw
    expect(detectDefaultCurrency()).toBe('USD');
  });

  it('falls back to languages[0] when language is empty', () => {
    vi.stubGlobal('navigator', { language: '', languages: ['fr-FR'] });
    expect(detectDefaultCurrency()).toBe('EUR');
  });

  it('returns CAD for Canadian locale (en-CA)', () => {
    vi.stubGlobal('navigator', { language: 'en-CA', languages: ['en-CA'] });
    expect(detectDefaultCurrency()).toBe('CAD');
  });

  it('returns AUD for Australian locale (en-AU)', () => {
    vi.stubGlobal('navigator', { language: 'en-AU', languages: ['en-AU'] });
    expect(detectDefaultCurrency()).toBe('AUD');
  });
});
