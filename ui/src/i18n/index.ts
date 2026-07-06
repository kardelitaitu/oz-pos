import { FluentBundle, FluentResource } from '@fluent/bundle';
import sharedFtl from '../locales/shared.ftl?raw';
import salesFtl from '../locales/sales.ftl?raw';
import productsFtl from '../locales/products.ftl?raw';
import settingsFtl from '../locales/settings.ftl?raw';
import staffFtl from '../locales/staff.ftl?raw';
import customersFtl from '../locales/customers.ftl?raw';
import taxFtl from '../locales/tax.ftl?raw';
import currencyFtl from '../locales/currency.ftl?raw';
import inventoryFtl from '../locales/inventory.ftl?raw';
import tablesFtl from '../locales/tables.ftl?raw';
import terminalsFtl from '../locales/terminals.ftl?raw';
import offlineFtl from '../locales/offline.ftl?raw';
import bundlesFtl from '../locales/bundles.ftl?raw';
import promotionsFtl from '../locales/promotions.ftl?raw';
import kdsFtl from '../locales/kds.ftl?raw';
import kioskFtl from '../locales/kiosk.ftl?raw';
import loyaltyFtl from '../locales/loyalty.ftl?raw';
import shiftsFtl from '../locales/shifts.ftl?raw';
import reportsFtl from '../locales/reports.ftl?raw';
import multiStoreFtl from '../locales/multi-store.ftl?raw';
import stockTransfersFtl from '../locales/stock-transfers.ftl?raw';
import idFTL from './id.ftl?raw';

export type LocaleCode = 'en' | 'id';

const enFTL = [
  sharedFtl,
  salesFtl,
  productsFtl,
  settingsFtl,
  staffFtl,
  customersFtl,
  taxFtl,
  currencyFtl,
  inventoryFtl,
  tablesFtl,
  terminalsFtl,
  offlineFtl,
  bundlesFtl,
  promotionsFtl,
  kdsFtl,
  kioskFtl,
  loyaltyFtl,
  shiftsFtl,
  reportsFtl,
  multiStoreFtl,
  stockTransfersFtl,
].join('\n');

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
  const labels: Record<LocaleCode, string> = { en: 'locale-en', id: 'locale-id' };
  return labels[locale];
}
