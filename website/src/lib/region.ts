/**
 * Region helper — reads/writes the user's selected region from localStorage.
 *
 * The region determines:
 * 1. Which pricing to show (USD for global, IDR for Indonesia)
 * 2. Which payment provider to use (Paddle for global, Midtrans for Indonesia)
 *
 * Set during signup (SignupForm.tsx) and persisted in localStorage as
 * `oz_region`. Falls back to 'global' when unset.
 */
export type Region = 'global' | 'id';

const STORAGE_KEY = 'oz_region';

export function getRegion(): Region {
  if (typeof window === 'undefined') return 'global';
  return (localStorage.getItem(STORAGE_KEY) as Region) || 'global';
}

export function setRegion(region: Region): void {
  if (typeof window !== 'undefined') {
    localStorage.setItem(STORAGE_KEY, region);
  }
}

export function isIndonesia(): boolean {
  return getRegion() === 'id';
}
