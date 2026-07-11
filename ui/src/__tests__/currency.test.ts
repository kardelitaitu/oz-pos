import { describe, expect, it, beforeEach } from 'vitest';

// Mock navigator before module import using Object.defineProperty
const mockNavigator = {
  language: 'en-US',
  languages: ['en-US'] as string[],
};

Object.defineProperty(globalThis, 'navigator', {
  value: mockNavigator,
  writable: true,
  configurable: true,
});

// Dynamic import to ensure the mock is in place
const { detectDefaultCurrency } = await import('@/utils/currency');

describe('detectDefaultCurrency', () => {
  beforeEach(() => {
    mockNavigator.language = 'en-US';
    mockNavigator.languages = ['en-US'];
  });

  // ── Known regions ──

  it('returns IDR for Indonesian locale (id-ID)', () => {
    mockNavigator.language = 'id-ID';
    expect(detectDefaultCurrency()).toBe('IDR');
  });

  it('returns USD for US locale (en-US)', () => {
    mockNavigator.language = 'en-US';
    expect(detectDefaultCurrency()).toBe('USD');
  });

  it('returns GBP for UK locale (en-GB)', () => {
    mockNavigator.language = 'en-GB';
    expect(detectDefaultCurrency()).toBe('GBP');
  });

  it('returns EUR for German locale (de-DE)', () => {
    mockNavigator.language = 'de-DE';
    expect(detectDefaultCurrency()).toBe('EUR');
  });

  it('returns EUR for French locale (fr-FR)', () => {
    mockNavigator.language = 'fr-FR';
    expect(detectDefaultCurrency()).toBe('EUR');
  });

  it('returns JPY for Japanese locale (ja-JP)', () => {
    mockNavigator.language = 'ja-JP';
    expect(detectDefaultCurrency()).toBe('JPY');
  });

  it('returns CNY for Chinese locale (zh-CN)', () => {
    mockNavigator.language = 'zh-CN';
    expect(detectDefaultCurrency()).toBe('CNY');
  });

  it('returns KRW for Korean locale (ko-KR)', () => {
    mockNavigator.language = 'ko-KR';
    expect(detectDefaultCurrency()).toBe('KRW');
  });

  it('returns CAD for Canadian locale (en-CA)', () => {
    mockNavigator.language = 'en-CA';
    expect(detectDefaultCurrency()).toBe('CAD');
  });

  it('returns AUD for Australian locale (en-AU)', () => {
    mockNavigator.language = 'en-AU';
    expect(detectDefaultCurrency()).toBe('AUD');
  });

  it('returns SGD for Singapore locale (en-SG)', () => {
    mockNavigator.language = 'en-SG';
    expect(detectDefaultCurrency()).toBe('SGD');
  });

  it('returns INR for Indian locale (hi-IN)', () => {
    mockNavigator.language = 'hi-IN';
    expect(detectDefaultCurrency()).toBe('INR');
  });

  it('returns BRL for Brazilian locale (pt-BR)', () => {
    mockNavigator.language = 'pt-BR';
    expect(detectDefaultCurrency()).toBe('BRL');
  });

  // ── Single-language codes (no region subtag) ──

  it('handles single language code "id" → IDR', () => {
    mockNavigator.language = 'id';
    // 'id' → 'ID' → REGION_TO_CURRENCY['ID'] = 'IDR'
    expect(detectDefaultCurrency()).toBe('IDR');
  });

  it('handles single language code "de" → EUR', () => {
    mockNavigator.language = 'de';
    // 'de' → 'DE' → REGION_TO_CURRENCY['DE'] = 'EUR'
    expect(detectDefaultCurrency()).toBe('EUR');
  });

  it('falls back to USD for single code not matching any region', () => {
    mockNavigator.language = 'en';
    // 'en' → 'EN' → not in map → fallback 'USD'
    expect(detectDefaultCurrency()).toBe('USD');
  });

  // ── Unknown / fallback ──

  it('falls back to USD for unknown region (xx-XX)', () => {
    mockNavigator.language = 'xx-XX';
    expect(detectDefaultCurrency()).toBe('USD');
  });

  it('maps NG (Nigeria) region to NGN', () => {
    mockNavigator.language = 'en-NG';
    expect(detectDefaultCurrency()).toBe('NGN');
  });

  it('falls back to USD for completely unknown locale (zz-ZZ)', () => {
    mockNavigator.language = 'zz-ZZ';
    expect(detectDefaultCurrency()).toBe('USD');
  });

  // ── European countries → EUR ──

  it('returns EUR for Italian locale', () => {
    mockNavigator.language = 'it-IT';
    expect(detectDefaultCurrency()).toBe('EUR');
  });

  it('returns EUR for Spanish locale', () => {
    mockNavigator.language = 'es-ES';
    expect(detectDefaultCurrency()).toBe('EUR');
  });

  it('returns EUR for Dutch locale', () => {
    mockNavigator.language = 'nl-NL';
    expect(detectDefaultCurrency()).toBe('EUR');
  });

  // ── Edge cases ──

  it('handles navigator.languages fallback when language is empty', () => {
    mockNavigator.language = '';
    mockNavigator.languages = ['fr-FR'];
    expect(detectDefaultCurrency()).toBe('EUR');
  });

  it('returns USD when both language and languages are unavailable', () => {
    mockNavigator.language = '';
    mockNavigator.languages = [];
    expect(detectDefaultCurrency()).toBe('USD');
  });
});
