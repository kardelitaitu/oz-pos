/**
 * Segmented-trial vertical detection (C2.1, subscription-tiers.md §4).
 *
 * The license server reads `trial_vertical` on trial-key activation to mint
 * the right trial: blank/unset → 14-day Plus, `restaurant`/`cafe` → 14-day
 * Pro, `enterprise_referral` → 30-day Pro. This module detects the vertical
 * a user arrived from — the website vertical landing pages carry it as a
 * `?v=` query param (e.g. `/en/untuk-kafe?v=kafe`) that survives the handoff
 * into the app's webview in dev/web builds, and the activation screen passes
 * the normalized value through to `activateLicense()`.
 *
 * Website vertical keys are normalized to the server contract: kafe/restoran
 * (and the raw English values) → `restaurant`; everything else (warung,
 * minimarket, retail, unknown, absent) → `` (the general 14-day Plus trial).
 */

/** Normalize a raw vertical value to the license-server contract. */
export function normalizeTrialVertical(raw: string | null | undefined): string {
  const v = (raw ?? '').trim().toLowerCase();
  switch (v) {
    case 'kafe':
    case 'restoran':
    case 'restaurant':
    case 'cafe':
    case 'coffee':
      return 'restaurant';
    case 'enterprise_referral':
    case 'enterprise':
    case 'referral':
      return 'enterprise_referral';
    default:
      // warung / minimarket / retail / unknown / absent → general Plus trial.
      return '';
  }
}

/**
 * Read the vertical from the current page URL — `?v=<vertical>` (the
 * website landing-page convention) or the `?vertical=<vertical>` alias.
 * Returns the normalized contract value ('' when unset or unknown).
 */
export function detectTrialVertical(search: string = window.location.search): string {
  const params = new URLSearchParams(search);
  const raw = params.get('v') ?? params.get('vertical') ?? '';
  return normalizeTrialVertical(raw);
}

/** True when the current page URL carries a detectable trial vertical. */
export function hasTrialVertical(search: string = window.location.search): boolean {
  return detectTrialVertical(search) !== '';
}
