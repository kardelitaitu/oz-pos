import { FluentBundle, FluentResource } from '@fluent/bundle';
import enFTL from './en.ftl?raw';
import idFTL from './id.ftl?raw';

export type LocaleCode = 'en' | 'id';

const RESOURCES: Record<LocaleCode, string> = {
  en: enFTL,
  id: idFTL,
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
  return ['en', 'id'];
}

export function getLocaleLabel(locale: LocaleCode): string {
  const labels: Record<LocaleCode, string> = { en: 'English', id: 'Bahasa Indonesia' };
  return labels[locale];
}
