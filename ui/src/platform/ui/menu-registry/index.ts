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

// ── Section names ────────────────────────────────────────────────────

/** Sidebar section identifiers for grouping navigation items. */
export type SectionName =
  | 'operations'
  | 'sales'
  | 'products'
  | 'finance'
  | 'customers'
  | 'reports'
  | 'management'
  | 'inventory'
  | 'settings'
  | 'dev';

/** Maps each section name to its Fluent i18n label key. */
export const SECTION_LABELS: Record<SectionName, string> = {
  operations: 'nav-section-operations',
  sales: 'nav-section-sales',
  products: 'nav-section-products',
  finance: 'nav-section-finance',
  customers: 'nav-section-customers',
  reports: 'nav-section-reports',
  management: 'nav-section-management',
  inventory: 'nav-section-inventory',
  settings: 'nav-section-settings',
  dev: 'nav-section-dev',
};

// ── Types ──────────────────────────────────────────────────────────

/** A navigation item registered for the sidebar menu. */
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
  /** Section this nav item belongs to for sidebar grouping. */
  section?: SectionName;
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
  const role = userRole.toLowerCase();
  const isOwner = role === 'owner' || role === 'role-owner';
  const isManager = isOwner ||
    role === 'manager' ||
    role === 'role-manager' ||
    role === 'staff' ||
    role === 'role-staff';

  if (required === 'owner') return isOwner;
  return isManager;
}

/**
 * Clear all registrations (useful for testing).
 */
export function clearNavItems(): void {
  navItems.length = 0;
}
