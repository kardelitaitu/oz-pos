/**
 * KDS card colour system.
 *
 * Provides per-theme default colours for card headers, status badges,
 * and action buttons — matching the prototype's `DEFAULT_COLORS_DARK`
 * and `DEFAULT_COLORS_LIGHT` palettes.
 *
 * `contrastText(bg)` returns a legible foreground colour (dark or light)
 * for any background, using perceived luminance.
 */

/** Per-theme colour config — keys match the prototype's colour slots. */
export interface KdsCardColors {
  /** Dine-in order header colour. */
  dinein: string;
  /** Takeaway order header colour. */
  takeaway: string;
  /** Rush/priority badge colour. */
  rush: string;
  /** Processing (in-progress) badge colour. */
  processing: string;
  /** Prepared (done) badge colour. */
  prepared: string;
  /** Pause action button colour. */
  pause: string;
  /** Resume action button colour. */
  resume: string;
  /** Complete (mark served) button colour. */
  complete: string;
}

/** Dark theme default colours — vibrant, high-chroma for dark surfaces. */
export const DEFAULT_COLORS_DARK: KdsCardColors = {
  dinein: '#22c55e',
  takeaway: '#147EFB',
  rush: '#ef4444',
  processing: '#f59e0b',
  prepared: '#22c55e',
  pause: '#f59e0b',
  resume: '#147EFB',
  complete: '#4ade80',
};

/** Light theme default colours — muted, desaturated for white surfaces. */
export const DEFAULT_COLORS_LIGHT: KdsCardColors = {
  dinein: '#89a1c8',
  takeaway: '#9484b8',
  rush: '#f04242',
  processing: '#89a1c8',
  prepared: '#242424',
  pause: '#dcdfe5',
  resume: '#3b4972',
  complete: '#a72525',
};

/**
 * Calculate a legible foreground colour for a given background.
 *
 * Uses perceived luminance (ITU-R BT.601) — returns dark text for
 * light backgrounds and light text for dark backgrounds.
 *
 * @param hex - Background colour as `#rrggbb`
 * @returns `'#1a1a1a'` (dark) or `'#e6e6e6'` (light)
 */
export function contrastText(hex: string): string {
  const h = hex.replace('#', '');
  const r = parseInt(h.substring(0, 2), 16) / 255;
  const g = parseInt(h.substring(2, 4), 16) / 255;
  const b = parseInt(h.substring(4, 6), 16) / 255;
  const lum = 0.299 * r + 0.587 * g + 0.114 * b;
  return lum > 0.55 ? '#1a1a1a' : '#e6e6e6';
}
