// ── Settings: Store, Receipt, Setup Wizard, Feature Flags ──────────

import { invoke } from '@tauri-apps/api/core';

// ── Receipt Settings ─────────────────────────────────────────────

export interface ReceiptSettingsDto {
  showCurrency: boolean;
  decimalSeparator: string;
  showTax: boolean;
  footer: string;
  paperWidth: string;
}

export const getReceiptSettings = (): Promise<ReceiptSettingsDto> =>
  invoke<ReceiptSettingsDto>('get_receipt_settings');

export const setReceiptSettings = (args: ReceiptSettingsDto): Promise<void> =>
  invoke<void>('set_receipt_settings', { args });

// ── Store Settings ───────────────────────────────────────────────

export interface StoreSettingsDto {
  name: string;
  address: string;
  taxId: string;
}

export const getStoreSettings = (): Promise<StoreSettingsDto> =>
  invoke<StoreSettingsDto>('get_store_settings');

export const setStoreSettings = (args: StoreSettingsDto): Promise<void> =>
  invoke<void>('set_store_settings', { args });

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

export const getSetupStatus = (): Promise<SetupStatus> =>
  invoke<SetupStatus>('get_setup_status');

// ── Feature Flags ────────────────────────────────────────────────

export interface EnabledFeaturesResult {
  features: string[];
}

export const getEnabledFeatures = (): Promise<EnabledFeaturesResult> =>
  invoke<EnabledFeaturesResult>('get_enabled_features');
