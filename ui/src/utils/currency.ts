/**
 * Map a region/language code to its most likely default currency.
 *
 * Uses `navigator.language` (system/browser locale) to choose a sensible
 * default. Falls back to `"USD"` when no mapping exists.
 */

const REGION_TO_CURRENCY: Record<string, string> = {
  ID: 'IDR',
  US: 'USD',
  GB: 'GBP',
  DE: 'EUR',
  FR: 'EUR',
  IT: 'EUR',
  ES: 'EUR',
  NL: 'EUR',
  BE: 'EUR',
  AT: 'EUR',
  PT: 'EUR',
  IE: 'EUR',
  FI: 'EUR',
  GR: 'EUR',
  JP: 'JPY',
  CN: 'CNY',
  KR: 'KRW',
  IN: 'INR',
  CA: 'CAD',
  AU: 'AUD',
  NZ: 'NZD',
  CH: 'CHF',
  SE: 'SEK',
  NO: 'NOK',
  DK: 'DKK',
  SG: 'SGD',
  HK: 'HKD',
  MY: 'MYR',
  TH: 'THB',
  PH: 'PHP',
  VN: 'VND',
  MX: 'MXN',
  BR: 'BRL',
  ZA: 'ZAR',
  RU: 'RUB',
  TR: 'TRY',
  SA: 'SAR',
  AE: 'AED',
  IL: 'ILS',
  PL: 'PLN',
  CZ: 'CZK',
  HU: 'HUF',
  CL: 'CLP',
  CO: 'COP',
  PE: 'PEN',
  AR: 'ARS',
  NG: 'NGN',
  KE: 'KES',
  EG: 'EGP',
};

/**
 * Detect the user's likely default currency from their browser/system locale.
 *
 * Extracts the region subtag from `navigator.language` (e.g. `"id-ID"` → `"ID"`,
 * `"en-US"` → `"US"`, `"de"` → `"DE"`) and looks up the corresponding currency.
 * Falls back to `"USD"` if no mapping exists.
 */
export function detectDefaultCurrency(): string {
  try {
    const raw = navigator.language || navigator.languages?.[0] || 'en-US';
    // Parse locale: "id-ID", "en-US", "de-DE", or just "id", "en", "de"
    const parts = raw.split('-');
    // The region is the LAST subtag that is exactly 2 uppercase letters or
    // the whole string if it's just a language code like "id" or "en".
    const region = parts.length > 1
      ? parts[parts.length - 1]!.toUpperCase()
      : parts[0]!.toUpperCase();

    return REGION_TO_CURRENCY[region] ?? 'USD';
  } catch {
    return 'USD';
  }
}
