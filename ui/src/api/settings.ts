// ── Settings: Store, Receipt, Setup Wizard, Feature Flags ──────────

import { invoke } from '@tauri-apps/api/core';

// ── Receipt Settings ─────────────────────────────────────────────

/** Receipt print layout and formatting settings. */
export interface ReceiptSettingsDto {
  showCurrency: boolean;
  decimalSeparator: string;
  showTax: boolean;
  footer: string;
  paperWidth: string;
  showTableNumber: boolean;
  marginTop: number;
  marginBottom: number;
  marginLeft: number;
  marginRight: number;
}

/** Get the current receipt settings. */
export const getReceiptSettings = (): Promise<ReceiptSettingsDto> =>
  invoke<ReceiptSettingsDto>('get_receipt_settings');

/** Update the receipt settings. */
export const setReceiptSettings = (args: ReceiptSettingsDto, userId: string): Promise<void> =>
  invoke<void>('set_receipt_settings', { args, userId });

/** Set receipt settings (scoped — ADR #7). */
export const setReceiptSettingsScoped = (sessionToken: string, args: ReceiptSettingsDto): Promise<void> =>
  invoke<void>('set_receipt_settings_scoped', { sessionToken, args });

// ── Store Settings ───────────────────────────────────────────────

/** Store-level settings (name, address, currency, etc). */
export interface StoreSettingsDto {
  name: string;
  address: string;
  taxId: string;
  currency: string;
  branch: string;
  logo?: string;
}

/** Get the store settings. */
export const getStoreSettings = (): Promise<StoreSettingsDto> =>
  invoke<StoreSettingsDto>('get_store_settings');

/** Update the store settings. */
export const setStoreSettings = (args: StoreSettingsDto, userId: string): Promise<void> =>
  invoke<void>('set_store_settings', { args, userId });

/** Set store settings (scoped — ADR #7). */
export const setStoreSettingsScoped = (sessionToken: string, args: StoreSettingsDto): Promise<void> =>
  invoke<void>('set_store_settings_scoped', { sessionToken, args });

// ── Credit Settings ───────────────────────────────────────────

/** Credit / tab sale settings for the store. */
export interface CreditSettingsDto {
  enabled: boolean;
  reminderIntervalHours: number;
  maxLimitMinor: number;
}

/** A credit (tab) sale awaiting settlement. */
export interface CreditSaleDto {
  saleId: string;
  customerName: string;
  totalMinor: number;
  currency: string;
  createdAt: string;
  settledAt: string | null;
  cashierName: string;
}

/** Get the credit / tab sale settings. */
export const getCreditSettings = (): Promise<CreditSettingsDto> =>
  invoke<CreditSettingsDto>('get_credit_settings');

/** Update the credit / tab sale settings. */
export const setCreditSettings = (args: CreditSettingsDto, userId: string): Promise<void> =>
  invoke<void>('set_credit_settings', { args, userId });

/** Set credit settings (scoped — ADR #7). */
export const setCreditSettingsScoped = (sessionToken: string, args: CreditSettingsDto): Promise<void> =>
  invoke<void>('set_credit_settings_scoped', { sessionToken, args });

/** List all credit (tab) sales awaiting settlement. */
export const listCreditSales = (): Promise<CreditSaleDto[]> =>
  invoke<CreditSaleDto[]>('list_credit_sales');

/** Settle (mark as paid) a credit sale. */
export const settleCredit = (saleId: string, userId: string): Promise<void> =>
  invoke<void>('settle_credit', { saleId, userId });

/** Settle a credit sale (scoped — ADR #7). */
export const settleCreditScoped = (sessionToken: string, saleId: string): Promise<void> =>
  invoke<void>('settle_credit_scoped', { sessionToken, saleId });

// ── Hardware Settings (printer + scanner) ─────────────────────

/** Hardware configuration for printer and scanner devices. */
export interface HardwareSettingsDto {
  printerConnection: string;
  printerDevicePath: string;
  printerPaperSize: string;
  scannerDeviceId: string;
  scannerInputMode: string;
}

/** Get the hardware settings (printer, scanner). */
export const getHardwareSettings = (): Promise<HardwareSettingsDto> =>
  invoke<HardwareSettingsDto>('get_hardware_settings');

/** Update the hardware settings. */
export const setHardwareSettings = (args: HardwareSettingsDto, userId: string): Promise<void> =>
  invoke<void>('set_hardware_settings', { args, userId });

/** Set hardware settings (scoped — ADR #7). */
export const setHardwareSettingsScoped = (sessionToken: string, args: HardwareSettingsDto): Promise<void> =>
  invoke<void>('set_hardware_settings_scoped', { sessionToken, args });

// ── Setup Wizard ─────────────────────────────────────────────────

/** Arguments for completing the initial setup wizard. */
export interface CompleteSetupArgs {
  preset: string;
  features: string[];
  default_currency?: string;
}

/** Whether the initial setup wizard has been completed. */
export interface SetupStatus {
  completed: boolean;
  preset: string | null;
}

/** Complete the initial setup wizard with a preset and enabled features. */
export const completeSetup = (args: CompleteSetupArgs): Promise<void> =>
  invoke<void>('complete_setup', { args });

/** Dismiss the setup wizard without completing it. */
export const dismissSetupWizard = (): Promise<void> =>
  invoke<void>('dismiss_setup_wizard');

/** Get the current setup wizard completion status. */
export const getSetupStatus = (): Promise<SetupStatus> =>
  invoke<SetupStatus>('get_setup_status');

/** Seed default roles for the store resolved from a session token. Returns the number of roles created. ADR #7. */
export const seedDefaultRolesScoped = (sessionToken: string): Promise<number> =>
  invoke<number>('seed_default_roles_scoped', { sessionToken });

// ── Feature Flags ────────────────────────────────────────────────

/** The set of feature flags that are currently enabled. */
export interface EnabledFeaturesResult {
  features: string[];
}

/** Get the list of enabled feature flags. */
export const getEnabledFeatures = (): Promise<EnabledFeaturesResult> =>
  invoke<EnabledFeaturesResult>('get_enabled_features');

// ── User Preferences ─────────────────────────────────────────

/** A single user preference key-value pair. */
export interface UserPrefEntry {
  key: string;
  value: string;
}

/** Get all preferences for a given user. */
export const getUserPreferences = (userId: string): Promise<Record<string, string>> =>
  invoke<Record<string, string>>('get_user_preferences', { userId });

/** Get user preferences (scoped — ADR #7). Uses session.user_id for lookup. */
export const getUserPreferencesScoped = (sessionToken: string): Promise<Record<string, string>> =>
  invoke<Record<string, string>>('get_user_preferences_scoped', { sessionToken });

/** Set preferences for a given user. */
export const setUserPreferences = (userId: string, prefs: UserPrefEntry[]): Promise<void> =>
  invoke<void>('set_user_preferences', { userId, prefs });

/** Set user preferences (scoped — ADR #7). Uses session.user_id for write. */
export const setUserPreferencesScoped = (sessionToken: string, prefs: UserPrefEntry[]): Promise<void> =>
  invoke<void>('set_user_preferences_scoped', { sessionToken, prefs });
