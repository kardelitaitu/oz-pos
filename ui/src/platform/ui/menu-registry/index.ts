/**
 * Menu Registry — modules register navigation items here so the
 * AppLayout sidebar can render them dynamically.
 *
 * @example
 * ```tsx
 * import { registerNavItem } from '@/platform/ui/menu-registry';
 *
 * registerNavItem({
 *   route: 'products',
 *   label: 'Products',
 *   feature: 'simple-retail',
 *   icon: <ProductIcon />,
 * });
 * ```
 */

import type { ReactNode } from 'react';
import type { RequiredRole } from '@/platform/ui/page-registry';

// ── Types ──────────────────────────────────────────────────────────

export interface NavItemRegistration {
  /** Route name this nav item navigates to. */
  route: string;
  /** Human-readable label displayed in the sidebar. */
  label: string;
  /** FTL i18n key for the label. Falls back to `label` when not found. */
  i18nKey?: string;
  /** Optional SVG icon element. */
  icon?: ReactNode;
  /** Optional feature key that must be enabled for this item to appear. */
  feature?: string;
  /** Optional role required to see this nav item. 'manager' includes owner. */
  requiredRole?: RequiredRole;
  /** Optional section label to group nav items (e.g. "App", "Management"). */
  section?: string;
}

// ── Registry ───────────────────────────────────────────────────────

const navItems: NavItemRegistration[] = [];

/**
 * Register a navigation item. Items are displayed in registration order.
 */
export function registerNavItem(item: NavItemRegistration): void {
  navItems.push(item);
}

/**
 * Get all registered nav items (in registration order), optionally
 * filtered by enabled features and user role.
 * If `userRole` is omitted, role gating is skipped.
 */
export function getNavItems(
  enabledFeatures?: Set<string>,
  userRole?: string,
): NavItemRegistration[] {
  return navItems.filter((item) => {
    if (item.feature && enabledFeatures && !enabledFeatures.has(item.feature)) {
      return false;
    }
    if (item.requiredRole && !hasNavRole(userRole, item.requiredRole)) {
      return false;
    }
    return true;
  });
}

function hasNavRole(userRole: string | undefined, required: RequiredRole): boolean {
  if (!userRole) return false;
  if (required === 'owner') return userRole === 'owner';
  return userRole === 'manager' || userRole === 'owner';
}

/**
 * Clear all registrations (useful for testing).
 */
export function clearNavItems(): void {
  navItems.length = 0;
}
