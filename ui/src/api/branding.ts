// ── Brand / White-label API ───────────────────────────────────────

import { loggedInvoke } from '@/utils/logged-invoke';

/** Brand and white-label settings for the store. */
export interface BrandSettings {
  primary_colour: string;
  logo_path: string | null;
  store_name: string;
}

/** Get the current brand settings. */
export const getBrandSettings = (): Promise<BrandSettings> =>
  loggedInvoke<BrandSettings>('get_brand_settings');

/** Set the brand primary colour. */
export const setBrandPrimaryColour = (colour: string): Promise<void> =>
  loggedInvoke<void>('set_brand_primary_colour', { colour });

/** Set the brand logo file path. */
export const setBrandLogoPath = (path: string): Promise<void> =>
  loggedInvoke<void>('set_brand_logo_path', { path });

/** Set the store display name for branding. */
export const setBrandStoreName = (name: string): Promise<void> =>
  loggedInvoke<void>('set_brand_store_name', { name });

/** Open a file picker dialog to select a logo image. Returns the chosen path or null. */
export const pickLogoFile = (): Promise<string | null> =>
  loggedInvoke<string | null>('pick_logo_file');
