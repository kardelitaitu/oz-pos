// ── Settings: Store, Receipt, Setup Wizard, Feature Flags ──────────

import { invoke } from '@tauri-apps/api/core';

// ── Receipt Settings ─────────────────────────────────────────────

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

export const getReceiptSettings = (): Promise<ReceiptSettingsDto> =>
  invoke<ReceiptSettingsDto>('get_receipt_settings');

export const setReceiptSettings = (args: ReceiptSettingsDto, userId: string): Promise<void> =>
  invoke<void>('set_receipt_settings', { args, userId });

// ── Store Settings ───────────────────────────────────────────────

export interface StoreSettingsDto {
  name: string;
  address: string;
  taxId: string;
  currency: string;
  branch: string;
  logo: string;
}

export const getStoreSettings = (): Promise<StoreSettingsDto> =>
  invoke<StoreSettingsDto>('get_store_settings');

export const setStoreSettings = (args: StoreSettingsDto, userId: string): Promise<void> =>
  invoke<void>('set_store_settings', { args, userId });

// ── Credit Settings ───────────────────────────────────────────

export interface CreditSettingsDto {
  enabled: boolean;
  reminderIntervalHours: number;
  maxLimitMinor: number;
}

export interface CreditSaleDto {
  saleId: string;
  customerName: string;
  totalMinor: number;
  currency: string;
  createdAt: string;
  settledAt: string | null;
  cashierName: string;
}

export const getCreditSettings = (): Promise<CreditSettingsDto> =>
  invoke<CreditSettingsDto>('get_credit_settings');

export const setCreditSettings = (args: CreditSettingsDto, userId: string): Promise<void> =>
  invoke<void>('set_credit_settings', { args, userId });

export const listCreditSales = (): Promise<CreditSaleDto[]> =>
  invoke<CreditSaleDto[]>('list_credit_sales');

export const settleCredit = (saleId: string, userId: string): Promise<void> =>
  invoke<void>('settle_credit', { saleId, userId });

// ── Hardware Settings (printer + scanner) ─────────────────────

export interface HardwareSettingsDto {
  printerConnection: string;
  printerDevicePath: string;
  printerPaperSize: string;
  scannerDeviceId: string;
  scannerInputMode: string;
}

export const getHardwareSettings = (): Promise<HardwareSettingsDto> =>
  invoke<HardwareSettingsDto>('get_hardware_settings');

export const setHardwareSettings = (args: HardwareSettingsDto, userId: string): Promise<void> =>
  invoke<void>('set_hardware_settings', { args, userId });

// ── Setup Wizard ─────────────────────────────────────────────────

export interface CompleteSetupArgs {
  preset: string;
  features: string[];
}

export interface SetupStatus {
  completed: boolean;
  preset: string | null;
}

export const completeSetup = (args: CompleteSetupArgs): Promise<void> =>
  invoke<void>('complete_setup', { args });

export const dismissSetupWizard = (): Promise<void> =>
  invoke<void>('dismiss_setup_wizard');

export const getSetupStatus = (): Promise<SetupStatus> =>
  invoke<SetupStatus>('get_setup_status');

// ── Feature Flags ────────────────────────────────────────────────

export interface EnabledFeaturesResult {
  features: string[];
}

export const getEnabledFeatures = (): Promise<EnabledFeaturesResult> =>
  invoke<EnabledFeaturesResult>('get_enabled_features');

// ── User Preferences ─────────────────────────────────────────

export interface UserPrefEntry {
  key: string;
  value: string;
}

export const getUserPreferences = (userId: string): Promise<Record<string, string>> =>
  invoke<Record<string, string>>('get_user_preferences', { userId });

export const setUserPreferences = (userId: string, prefs: UserPrefEntry[]): Promise<void> =>
  invoke<void>('set_user_preferences', { userId, prefs });
