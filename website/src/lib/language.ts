/**
 * Language helper — reads/writes the user's preferred language from localStorage.
 *
 * When a user switches language via the locale switcher, their choice is saved
 * to localStorage as `oz_language`. On subsequent visits, if the URL doesn't
 * already have a locale prefix, the saved language is used to redirect.
 *
 * This does NOT override the URL — if the user visits /en/pricing directly,
 * they stay on EN. The preference only applies when navigating to locale-less
 * paths or on first visit.
 */
export type Language = 'en' | 'id';

const STORAGE_KEY = 'oz_language';

export function getPreferredLanguage(): Language | null {
  if (typeof window === 'undefined') return null;
  return (localStorage.getItem(STORAGE_KEY) as Language) || null;
}

export function setPreferredLanguage(lang: Language): void {
  if (typeof window !== 'undefined') {
    localStorage.setItem(STORAGE_KEY, lang);
  }
}
