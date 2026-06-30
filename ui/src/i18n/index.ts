import { FluentBundle, FluentResource } from '@fluent/bundle';
import enFTL from './en.ftl?raw';
import idFTL from './id.ftl?raw';
import thFTL from './th.ftl?raw';

export type LocaleCode = 'en' | 'id' | 'th';

const RESOURCES: Record<LocaleCode, string> = {
  en: enFTL,
  id: idFTL,
  th: thFTL,
};

const bundles = new Map<LocaleCode, FluentBundle>();

export function getBundle(locale: LocaleCode): FluentBundle {
  let bundle = bundles.get(locale);
  if (!bundle) {
    bundle = new FluentBundle(locale, { useIsolating: false });
    const resource = new FluentResource(RESOURCES[locale]);
    const errors = bundle.addResource(resource);
    if (errors.length > 0) {
      console.warn(`Fluent errors for ${locale}:`, errors);
    }
    bundles.set(locale, bundle);
  }
  return bundle;
}

export function getAvailableLocales(): LocaleCode[] {
  return ['en', 'id', 'th'];
}

export function getLocaleLabel(locale: LocaleCode): string {
  const labels: Record<LocaleCode, string> = { en: 'English', id: 'Bahasa Indonesia', th: 'ภาษาไทย' };
  return labels[locale];
}
