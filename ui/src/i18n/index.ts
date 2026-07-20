import { FluentBundle, FluentResource } from '@fluent/bundle';

// English domain files
import sharedEn from '../locales/shared.ftl?raw';
import salesEn from '../locales/sales.ftl?raw';
import productsEn from '../locales/products.ftl?raw';
import settingsEn from '../locales/settings.ftl?raw';
import staffEn from '../locales/staff.ftl?raw';
import customersEn from '../locales/customers.ftl?raw';
import taxEn from '../locales/tax.ftl?raw';
import currencyEn from '../locales/currency.ftl?raw';
import inventoryEn from '../locales/inventory.ftl?raw';
import tablesEn from '../locales/tables.ftl?raw';
import terminalsEn from '../locales/terminals.ftl?raw';
import offlineEn from '../locales/offline.ftl?raw';
import bundlesEn from '../locales/bundles.ftl?raw';
import promotionsEn from '../locales/promotions.ftl?raw';
import kdsEn from '../locales/kds.ftl?raw';
import kioskEn from '../locales/kiosk.ftl?raw';
import loyaltyEn from '../locales/loyalty.ftl?raw';
import shiftsEn from '../locales/shifts.ftl?raw';
import reportsEn from '../locales/reports.ftl?raw';
import multiStoreEn from '../locales/multi-store.ftl?raw';
import stockTransfersEn from '../locales/stock-transfers.ftl?raw';
import giftCardsEn from '../locales/gift-cards.ftl?raw';
import purchasingEn from '../locales/purchasing.ftl?raw';
import stockCountingEn from '../locales/stock-counting.ftl?raw';

// Indonesian domain files
import sharedId from '../locales/shared.id.ftl?raw';
import salesId from '../locales/sales.id.ftl?raw';
import productsId from '../locales/products.id.ftl?raw';
import settingsId from '../locales/settings.id.ftl?raw';
import staffId from '../locales/staff.id.ftl?raw';
import customersId from '../locales/customers.id.ftl?raw';
import taxId from '../locales/tax.id.ftl?raw';
import currencyId from '../locales/currency.id.ftl?raw';
import inventoryId from '../locales/inventory.id.ftl?raw';
import tablesId from '../locales/tables.id.ftl?raw';
import terminalsId from '../locales/terminals.id.ftl?raw';
import offlineId from '../locales/offline.id.ftl?raw';
import bundlesId from '../locales/bundles.id.ftl?raw';
import promotionsId from '../locales/promotions.id.ftl?raw';
import kdsId from '../locales/kds.id.ftl?raw';
import kioskId from '../locales/kiosk.id.ftl?raw';
import loyaltyId from '../locales/loyalty.id.ftl?raw';
import shiftsId from '../locales/shifts.id.ftl?raw';
import reportsId from '../locales/reports.id.ftl?raw';
import multiStoreId from '../locales/multi-store.id.ftl?raw';
import stockTransfersId from '../locales/stock-transfers.id.ftl?raw';
import giftCardsId from '../locales/gift-cards.id.ftl?raw';
import purchasingId from '../locales/purchasing.id.ftl?raw';
import stockCountingId from '../locales/stock-counting.id.ftl?raw';

// Thai domain files (scaffolding — professional translation pending)
import sharedTh from '../locales/shared.th.ftl?raw';
import salesTh from '../locales/sales.th.ftl?raw';
import productsTh from '../locales/products.th.ftl?raw';
import settingsTh from '../locales/settings.th.ftl?raw';
import staffTh from '../locales/staff.th.ftl?raw';
import customersTh from '../locales/customers.th.ftl?raw';
import taxTh from '../locales/tax.th.ftl?raw';
import currencyTh from '../locales/currency.th.ftl?raw';
import inventoryTh from '../locales/inventory.th.ftl?raw';
import tablesTh from '../locales/tables.th.ftl?raw';
import terminalsTh from '../locales/terminals.th.ftl?raw';
import offlineTh from '../locales/offline.th.ftl?raw';
import bundlesTh from '../locales/bundles.th.ftl?raw';
import promotionsTh from '../locales/promotions.th.ftl?raw';
import kdsTh from '../locales/kds.th.ftl?raw';
import kioskTh from '../locales/kiosk.th.ftl?raw';
import loyaltyTh from '../locales/loyalty.th.ftl?raw';
import shiftsTh from '../locales/shifts.th.ftl?raw';
import reportsTh from '../locales/reports.th.ftl?raw';
import multiStoreTh from '../locales/multi-store.th.ftl?raw';
import stockTransfersTh from '../locales/stock-transfers.th.ftl?raw';
import giftCardsTh from '../locales/gift-cards.th.ftl?raw';
import purchasingTh from '../locales/purchasing.th.ftl?raw';
import stockCountingTh from '../locales/stock-counting.th.ftl?raw';

/** Supported application locale codes. */
export type LocaleCode = 'en' | 'id' | 'th';

const enFTL = [
  sharedEn, salesEn, productsEn, settingsEn, staffEn,
  customersEn, taxEn, currencyEn, inventoryEn, tablesEn,
  terminalsEn, offlineEn, bundlesEn, promotionsEn, kdsEn,
  kioskEn, loyaltyEn, shiftsEn, reportsEn, multiStoreEn,
  stockTransfersEn, giftCardsEn, purchasingEn, stockCountingEn,
].join('\n');

const idFTL = [
  sharedId, salesId, productsId, settingsId, staffId,
  customersId, taxId, currencyId, inventoryId, tablesId,
  terminalsId, offlineId, bundlesId, promotionsId, kdsId,
  kioskId, loyaltyId, shiftsId, reportsId, multiStoreId,
  stockTransfersId, giftCardsId, purchasingId, stockCountingId,
].join('\n');

const thFTL = [
  sharedTh, salesTh, productsTh, settingsTh, staffTh,
  customersTh, taxTh, currencyTh, inventoryTh, tablesTh,
  terminalsTh, offlineTh, bundlesTh, promotionsTh, kdsTh,
  kioskTh, loyaltyTh, shiftsTh, reportsTh, multiStoreTh,
  stockTransfersTh, giftCardsTh, purchasingTh, stockCountingTh,
].join('\n');

const RESOURCES: Record<LocaleCode, string> = {
  en: enFTL,
  id: idFTL,
  th: thFTL,
};

const bundles = new Map<LocaleCode, FluentBundle>();

/** Get (or create) a cached FluentBundle for the given locale. */
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

/** Return the list of locale codes the application supports. */
export function getAvailableLocales(): LocaleCode[] {
  return ['en', 'id', 'th'];
}

/** Return the Fluent i18n key for a locale's display label. */
export function getLocaleLabel(locale: LocaleCode): string {
  const labels: Record<LocaleCode, string> = { en: 'locale-en', id: 'locale-id', th: 'locale-th' };
  return labels[locale];
}
