// ── Brand / White-label API ───────────────────────────────────────

import { invoke } from '@tauri-apps/api/core';

export interface BrandSettings {
  primary_colour: string;
  logo_path: string | null;
  store_name: string;
}

export const getBrandSettings = (): Promise<BrandSettings> =>
  invoke<BrandSettings>('get_brand_settings');

export const setBrandPrimaryColour = (colour: string): Promise<void> =>
  invoke<void>('set_brand_primary_colour', { colour });

export const setBrandLogoPath = (path: string): Promise<void> =>
  invoke<void>('set_brand_logo_path', { path });

export const setBrandStoreName = (name: string): Promise<void> =>
  invoke<void>('set_brand_store_name', { name });

export const pickLogoFile = (): Promise<string | null> =>
  invoke<string | null>('pick_logo_file');
