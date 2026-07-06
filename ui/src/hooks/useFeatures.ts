import { useState, useEffect, useMemo, useCallback } from 'react';
import { getEnabledFeatures } from '@/api/settings';

// ── Feature key constants (must match the Rust Feature enum kebab-case keys) ─

export const FEATURES = {
  SIMPLE_RETAIL: 'simple-retail',
  RESTAURANT: 'restaurant',
  CASH_PAYMENT: 'cash-payment',
  CARD_PAYMENT: 'card-payment',
  MULTI_CURRENCY: 'multi-currency',
  INVENTORY_TRACKING: 'inventory-tracking',
  PRODUCT_VARIANTS: 'product-variants',
  CATEGORIES_ENABLED: 'categories-enabled',
  STAFF_LOGIN: 'staff-login',
  STAFF_ROLES: 'staff-roles',
  SHIFT_MANAGEMENT: 'shift-management',
  AUDIT_LOG: 'audit-log',
  BARCODE_SCANNING: 'barcode-scanning',
  RECEIPT_PRINTING: 'receipt-printing',
  CASH_DRAWER: 'cash-drawer',
  CUSTOMER_DISPLAY: 'customer-display',
  NFC_READER: 'nfc-reader',
  USB_SCALE: 'usb-scale',
  DISCOUNT_ENGINE: 'discount-engine',
  TAX_ENGINE: 'tax-engine',
  LOYALTY_PROGRAM: 'loyalty-program',
  QUICK_RETURN: 'quick-return',
  PROMOTIONS_ENGINE: 'promotions-engine',
  PRODUCT_BUNDLES: 'product-bundles',
  SERIAL_TRACKING: 'serial-tracking',
  STOCK_TRANSFERS: 'stock-transfers',
  KITCHEN_DISPLAY: 'kitchen-display',
  TABLE_MANAGEMENT: 'table-management',
  SELF_SERVICE_KIOSK: 'self-service-kiosk',
  CLOUD_SYNC: 'cloud-sync',
  MULTI_STORE: 'multi-store',
  MULTI_TERMINAL: 'multi-terminal',
  REPORTING: 'reporting',
  ANALYTICS: 'analytics',
  EXPORT_IMPORT: 'export-import',
  PLUGIN_SYSTEM: 'plugin-system',
} as const;

export type FeatureKey = (typeof FEATURES)[keyof typeof FEATURES];

// ── Nav item to feature key mapping ──────────────────────────────────
//
// Which feature must be enabled for each nav route to appear.
// If a route is not listed here, it's always visible.

export const ROUTE_FEATURE: Record<string, FeatureKey | undefined> = {
  sales: FEATURES.SIMPLE_RETAIL,
  'stock-transfers': FEATURES.STOCK_TRANSFERS,
};

// ── Hook ─────────────────────────────────────────────────────────────

export interface UseFeaturesResult {
  /** Set of enabled feature keys (kebab-case). */
  enabled: Set<string>;
  /** Whether features are still loading from the backend. */
  loading: boolean;
  /** Whether the given feature key is enabled. */
  isEnabled: (key: string) => boolean;
  /** Filter an array of route names to only those whose feature is enabled. */
  filterRoutes: (routes: string[]) => string[];
  /** Error message if the IPC call failed. */
  error: string | null;
  /** True if features were loaded (even if empty set). */
  loaded: boolean;
}

/**
 * Load the store's enabled feature flags from the backend on mount.
 *
 * The hook returns an `isEnabled(key)` helper and a `filterRoutes` helper
 * so UI components can conditionally render based on feature flags.
 *
 * @example
 * ```tsx
 * const { isEnabled, loading, filterRoutes } = useFeatures();
 * if (loading) return <Spinner />;
 * if (isEnabled('cash-payment')) return <CashPaymentOption />;
 * ```
 */
export function useFeatures(): UseFeaturesResult {
  const [enabled, setEnabled] = useState<Set<string>>(new Set());
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    (async () => {
      try {
        const result = await getEnabledFeatures();
        if (!cancelled) {
          setEnabled(new Set(result.features));
        }
      } catch (err) {
        // IPC unavailable (e.g. running outside Tauri in dev).
        // Enable all features as a reasonable fallback so the UI is fully visible.
        if (!cancelled) {
          setError(
            err instanceof Error ? err.message : 'Failed to load features',
          );
          setEnabled(new Set(Object.values(FEATURES)));
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, []);

  const isEnabled = useCallback(
    (key: string): boolean => enabled.has(key),
    [enabled],
  );

  const filterRoutes = useCallback(
    (routes: string[]): string[] => {
      return routes.filter((route) => {
        const requiredFeature = ROUTE_FEATURE[route];
        // If no feature is required for this route, show it always.
        if (!requiredFeature) return true;
        return enabled.has(requiredFeature);
      });
    },
    [enabled],
  );

  const loaded = !loading;

  return useMemo(
    () => ({ enabled, loading, isEnabled, filterRoutes, error, loaded }),
    [enabled, loading, isEnabled, filterRoutes, error, loaded],
  );
}
