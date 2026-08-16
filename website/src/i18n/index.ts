import en from './en.json';
import id from './id.json';

export type Locale = 'en' | 'id';
export const locales: Locale[] = ['en', 'id'];

const dicts: Record<Locale, Record<string, unknown>> = { en, id };

/** Dot-path string lookup, e.g. `t('en', 'hero.title')`. Falls back to the key. */
export function t(locale: string, key: string): string {
  const value = resolve(locale, key);
  return typeof value === 'string' ? value : key;
}

/** Raw dict for structured content (arrays/objects), e.g. `dict(locale).features.items`. */
export function dict(locale: string): Record<string, unknown> {
  return dicts[(locale as Locale)] ?? en;
}

function resolve(locale: string, key: string): unknown {
  return key
    .split('.')
    .reduce<unknown>((acc, part) => (acc as Record<string, unknown> | undefined)?.[part], dict(locale));
}
