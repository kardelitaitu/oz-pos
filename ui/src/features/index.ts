import { registerSalesFeature } from './sales/register';
import { registerProductsFeature } from './products/register';
import { registerInventoryFeature } from './inventory/register';
import { registerCustomersFeature } from './customers/register';
import { registerGiftCardsFeature } from './gift-cards/register';
import { registerLoyaltyFeature } from './loyalty/register';
import { registerStaffFeature } from './staff/register';
import { registerTerminalsFeature } from './terminals/register';
import { registerStoresFeature } from './stores/register';
import { registerSettingsFeature } from './settings/register';
import { registerTaxFeature } from './tax/register';
import { registerCurrencyFeature } from './currency/register';
import { registerCategoriesFeature } from './categories/register';
import { registerAuditFeature } from './audit/register';
import { registerOfflineFeature } from './offline/register';
import { registerShiftsFeature } from './shifts/register';
import { registerReportsFeature } from './reports/register';
import { registerDesignFeature } from './design/register';
import { registerKdsFeature } from './kds/register';
import { registerKioskFeature } from './kiosk/register';
import { registerTablesFeature } from './tables/register';
import { registerPromotionsFeature } from './promotions/register';
import { registerPurchasingFeature } from './purchasing/register';
import { registerStockTransfersFeature } from './stock-transfers/register';

/**
 * Register all UI features, pages, navigation items, and widgets.
 */
export function registerAllFeatures() {
  registerSalesFeature();
  registerProductsFeature();
  registerInventoryFeature();
  registerCustomersFeature();
  registerGiftCardsFeature();
  registerLoyaltyFeature();
  registerStaffFeature();
  registerTerminalsFeature();
  registerStoresFeature();
  registerSettingsFeature();
  registerTaxFeature();
  registerCurrencyFeature();
  registerCategoriesFeature();
  registerAuditFeature();
  registerOfflineFeature();
  registerShiftsFeature();
  registerReportsFeature();
  registerDesignFeature();
  registerKdsFeature();
  registerKioskFeature();
  registerTablesFeature();
  registerPromotionsFeature();
  registerPurchasingFeature();
  registerStockTransfersFeature();
}
