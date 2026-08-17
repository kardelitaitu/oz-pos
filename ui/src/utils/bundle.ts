/**
 * Vertical-bundle detection (C3.2, subscription-tiers.md §3).
 *
 * The license server reads `bundle_id` on trial-key activation to unlock
 * the kds workspace type at the Plus tier ("restaurant_starter"). This
 * module detects the bundle a user arrived from — the website carries it as
 * a `?bundle=` query param (e.g. `/en/untuk-kafe?v=kafe&bundle=restaurant_starter`)
 * that survives the handoff into the app's webview in dev/web builds, and
 * the activation screen passes the normalized value through to
 * `activateLicense()`.
 *
 * Only "restaurant_starter" is recognized today; everything else
 * (unknown, absent) normalizes to `''` — a no-op bundle.
 */

/** Normalize a raw bundle id to the license-server contract. */
export function normalizeBundleId(raw: string | null | undefined): string {
  const v = (raw ?? '').trim().toLowerCase();
  return v === 'restaurant_starter' ? v : '';
}

/**
 * Read the bundle from the current page URL — `?bundle=<id>` (the website
 * landing-page convention). Returns the normalized contract value ('' when
 * unset or unknown).
 */
export function detectBundleId(search: string = window.location.search): string {
  const params = new URLSearchParams(search);
  return normalizeBundleId(params.get('bundle'));
}

/** True when the current page URL carries a detectable bundle. */
export function hasBundleId(search: string = window.location.search): boolean {
  return detectBundleId(search) !== '';
}
