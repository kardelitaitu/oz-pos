/**
 * Page Registry — modules register their screens here so App.tsx
 * can render them dynamically instead of a hardcoded switch.
 *
 * @example
 * ```tsx
 * import { registerPage } from '@/platform/ui/page-registry';
 * import PosScreen from './PosScreen';
 *
 * registerPage({
 *   route: 'sales',
 *   component: PosScreen,
 *   label: 'POS Terminal',
 *   feature: 'simple-retail',
 * });
 * ```
 */

import type { ComponentType } from 'react';

// ── Types ──────────────────────────────────────────────────────────

export type RequiredRole = 'manager' | 'owner';

export interface PageRegistration {
  /** Route name used for navigation (e.g. 'sales', 'products'). */
  route: string;
  /** The React component to render for this route. */
  component: ComponentType;
  /** Human-readable label for nav items. */
  label: string;
  /** Optional feature key that must be enabled for this page to appear. */
  feature?: string;
  /** Optional role required to access this page. 'manager' includes owner. */
  requiredRole?: RequiredRole;
  /** Optional SVG icon element for nav items. */
  icon?: React.ReactNode;
}

// ── Registry ───────────────────────────────────────────────────────

const pages = new Map<string, PageRegistration>();

/**
 * Register a page with the registry. Duplicate route names will be
 * overwritten by the last registration (allows feature modules to
 * override defaults).
 */
export function registerPage(registration: PageRegistration): void {
  pages.set(registration.route, registration);
}

/**
 * Get a registered page by route name.
 * Returns undefined if no page is registered for the route.
 */
export function getPage(route: string): PageRegistration | undefined {
  return pages.get(route);
}

/**
 * Check whether a page is accessible by the given user role.
 * Returns true if the page has no requiredRole, or the user's role satisfies it.
 */
export function isPageAccessible(
  registration: PageRegistration | undefined,
  userRole: string | undefined,
): boolean {
  if (!registration || !registration.requiredRole) return true;
  return hasRequiredRole(userRole, registration.requiredRole);
}

/**
 * Get all registered pages (in registration order).
 */
export function getAllPages(): PageRegistration[] {
  return Array.from(pages.values());
}

/**
 * Get pages that are enabled given the current feature set and user role.
 * If `enabledFeatures` is omitted, all pages are returned.
 * If `userRole` is omitted, role gating is skipped.
 */
export function getEnabledPages(
  enabledFeatures?: Set<string>,
  userRole?: string,
): PageRegistration[] {
  return Array.from(pages.values()).filter((p) => {
    if (p.feature && enabledFeatures && !enabledFeatures.has(p.feature)) {
      return false;
    }
    if (p.requiredRole && !hasRequiredRole(userRole, p.requiredRole)) {
      return false;
    }
    return true;
  });
}

/**
 * Check if the user's role satisfies a required role.
 * 'owner' satisfies 'manager' and 'owner'.
 * 'manager' satisfies 'manager' only.
 */
function hasRequiredRole(userRole: string | undefined, required: RequiredRole): boolean {
  if (!userRole) return false;
  if (required === 'owner') return userRole === 'owner';
  return userRole === 'manager' || userRole === 'owner';
}

/**
 * Clear all registrations (useful for testing).
 */
export function clearPages(): void {
  pages.clear();
}
