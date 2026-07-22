// ── Screen CSS extraction integrity tests ─────────────────────────
//
// Regression guard: for every screen component with companion
// stylesheet(s), we assert that:
//   1. Every className used in the TSX has a CSS rule defined
//   2. No className is duplicated across multiple files
//   3. No dead classes exist (soft warning)
//
// Add a new screen by appending an entry to the SCREENS array below.

import { describe, it, expect } from 'vitest';
import fs from 'fs';
import path from 'path';
import {
  extractClassSelectors,
  extractUsedClassNames,
} from './screenExtraction.utils';

// ── File layout ───────────────────────────────────────────────────
// This test lives at  ui/src/__tests__/screenExtraction.test.ts
// Screens + CSS live at ui/src/features/*/

const FEATURES_DIR = path.resolve(process.cwd(), 'src', 'features');

interface ScreenEntry {
  /** Display name for the test suite (e.g. "ProductLookupScreen"). */
  name: string;
  /** Path to the TSX file, relative to src/features/. */
  tsx: string;
  /** Path(s) to companion CSS files, relative to src/features/. */
  css: string[];
  /**
   * Class-name prefixes whose BEM‑like modifiers are constructed
   * at runtime via template literals or returned from helper
   * functions.  The static analysis cannot extract the full
   * modifier class names, so these prefixes are excluded from the
   * dead‑class check.
   *
   * Example: `['kds-column--', 'inv-adjust-stock--']`
   * would acknowledge classes like `kds-column--pending` and
   * `inv-adjust-stock--ok` as reachable at runtime even though
   * they never appear verbatim in the TSX source.
   */
  dynamicClassPrefixes?: string[];
  /**
   * Class names that are legitimately referenced in companion CSS
   * but belong to child / imported components rather than the
   * screen's own TSX. These are excluded from the dead‑class check.
   *
   * Example: `['card', 'tab-list']` for a page that uses a <Card>
   * component rendering a `.card` class internally.
   */
  externalClasses?: string[];
  /**
   * String values that the static `extractUsedClassNames` parser
   * falsely extracts as CSS class names because they appear inside
   * template-literal interpolations (e.g. `flashRows.has('backup')`).
   * Unlike `dynamicClassPrefixes` (which handles BEM-like dynamic
   * modifiers), these fragments are NOT real CSS classes — they are
   * function arguments, comparison values, or other string literals
   * that happen to look like class names to the regex parser.
   *
   * Entries here are excluded from the "used but not defined" check.
   */
  knownDynamicFragments?: string[];
}

const SCREENS: ScreenEntry[] = [
  // ── Products ──────────────────────────────────────────
  {
    name: 'ProductLookupScreen',
    tsx: 'products/ProductLookupScreen.tsx',
    css: ['products/ProductLookupScreen.css'],
    externalClasses: ['product-card', 'product-card--added', 'product-card--disabled'],
  },
  {
    name: 'ProductManagementScreen',
    tsx: 'products/ProductManagementScreen.tsx',
    css: ['products/ProductManagementScreen.css'],
    dynamicClassPrefixes: ['product-mgmt-type--'],
    // Classes used by child StockAlertPanel component rendered inside drawer
    externalClasses: ['stock-alert-panel', 'product-mgmt-alert-badge'],
  },
  {
    name: 'BundleManagementScreen',
    tsx: 'products/BundleManagementScreen.tsx',
    css: ['products/BundleManagementScreen.css'],
  },

  // ── Staff ─────────────────────────────────────────────
  {
    name: 'StaffManagementScreen',
    tsx: 'staff/StaffManagementScreen.tsx',
    css: ['staff/StaffManagementScreen.css'],
  },

  // ── Setup ─────────────────────────────────────────────
  {
    name: 'SetupWizard',
    tsx: 'setup/SetupWizard.tsx',
    css: ['setup/SetupWizard.css'],
  },

  // ── Customers ─────────────────────────────────────────
  {
    name: 'CustomerManagementScreen',
    tsx: 'customers/CustomerManagementScreen.tsx',
    css: ['customers/CustomerManagementScreen.css'],
  },

  // ── Inventory ─────────────────────────────────────────
  {
    name: 'InventoryAdjustmentScreen',
    tsx: 'inventory/InventoryAdjustmentScreen.tsx',
    css: ['inventory/InventoryAdjustmentScreen.css'],
    dynamicClassPrefixes: ['inv-adjust-stock--'],
  },

  // ── Auth ──────────────────────────────────────────────
  {
    name: 'StaffLoginScreen',
    tsx: 'auth/StaffLoginScreen.tsx',
    css: ['auth/StaffLoginScreen.css'],
    dynamicClassPrefixes: ['staff-login-logo', 'staff-login-card'],
    knownDynamicFragments: ['skeleton'],
  },

  // ── Audit ─────────────────────────────────────────────
  {
    name: 'AuditLogScreen',
    tsx: 'audit/AuditLogScreen.tsx',
    css: ['audit/AuditLogScreen.css'],
    dynamicClassPrefixes: ['audit-log-badge--'],
  },

  // ── Categories ────────────────────────────────────────
  {
    name: 'CategoryManagementScreen',
    tsx: 'categories/CategoryManagementScreen.tsx',
    css: ['categories/CategoryManagementScreen.css'],
  },

  // ── Currency ──────────────────────────────────────────
  {
    name: 'ExchangeRateScreen',
    tsx: 'currency/ExchangeRateScreen.tsx',
    css: ['currency/ExchangeRateScreen.css'],
  },

  // ── KDS ───────────────────────────────────────────────
  {
    name: 'KdsScreen',
    tsx: 'kds/KdsScreen.tsx',
    css: ['kds/KdsScreen.css'],
    dynamicClassPrefixes: ['kds-column--', 'kds-ticket', 'kds-workspace'],
    externalClasses: ['kds-empty'],
  },

  // ── Loyalty ───────────────────────────────────────────
  {
    name: 'LoyaltyManagementScreen',
    tsx: 'loyalty/LoyaltyManagementScreen.tsx',
    css: ['loyalty/LoyaltyManagementScreen.css'],
    dynamicClassPrefixes: ['loyalty-txn-type--'],
  },

  // ── Offline ───────────────────────────────────────────
  {
    name: 'OfflineQueueScreen',
    tsx: 'offline/OfflineQueueScreen.tsx',
    css: ['offline/OfflineQueueScreen.css'],
    dynamicClassPrefixes: ['status-'],
  },

  // ── Promotions ────────────────────────────────────────
  {
    name: 'PromotionManagementScreen',
    tsx: 'promotions/PromotionManagementScreen.tsx',
    css: ['promotions/PromotionManagementScreen.css'],
  },

  // ── Settings ──────────────────────────────────────────
  {
    name: 'SettingsPage',
    tsx: 'settings/SettingsPage.tsx',
    css: ['settings/SettingsPage.css'],
    knownDynamicFragments: [
      // Object-key strings inside template-literal interpolations that
      // the static class-name parser falsely extracts as CSS classes.
      'store-name',
      'address',
      'tax-id',
      'settings-sync-token-actions',
      'settings-sync-status-text',
      'topology',
    ],
    externalClasses: [
      'card',
      'tab-list',
      'tooltip-content',
      'feature-toggle',
      'data-mgmt',
      'staff-mgmt',
      'terminal-mgmt',
      'multi-store-dashboard',
      'audit-log',
      'offline-queue-screen',
      'shift-mgmt',
      'tax-config',
      'exchange-rate-config',
      'promo-mgmt',
      'mobile-open',
      'visible',
      'settings-topology-container',
      // Visibility-hidden modifier classes for revert button & save-dot.
      // These are constructed via template-literal class toggling in
      // SettingsPage.tsx, so the static parser can't extract them.
      'settings-btn-revert--hidden',
      'settings-save-dot--hidden',
      // Sync status classes used in SettingsPage.tsx
      'settings-sync-dot--err',
      'settings-sync-expiry-badge--good',
      'settings-sync-expiry-badge--warn',
      'settings-sync-expiry-badge--critical',
    ],
  },
  {
    name: 'DataManagementScreen',
    tsx: 'settings/DataManagementScreen.tsx',
    css: ['settings/DataManagementScreen.css'],
    dynamicClassPrefixes: ['data-mgmt-toast--'],
    knownDynamicFragments: [
      // Template-literal parameters inside flashRows.has() that the
      // static class-name parser falsely extracts as class names.
      'import-preview',
      'backup',
    ],
  },
  {
    name: 'FeatureToggleScreen',
    tsx: 'settings/FeatureToggleScreen.tsx',
    css: ['settings/FeatureToggleScreen.css'],
    dynamicClassPrefixes: [
      // Flash + checkmark classes constructed via template literals.
      // 'feature-toggle-item' covers both the base item class and
      // the --flash-enabled/--flash-disabled modifier variants.
      'feature-toggle-item',
      'feature-toggle-checkmark--',
    ],
  },

  // ── Shifts ────────────────────────────────────────────
  {
    name: 'ShiftManagementScreen',
    tsx: 'shifts/ShiftManagementScreen.tsx',
    css: ['shifts/ShiftManagementScreen.css'],
    dynamicClassPrefixes: ['shift-mgmt-status-badge--', 'shift-mgmt-close-info'],
  },

  // ── Stores ────────────────────────────────────────────
  {
    name: 'MultiStoreDashboardScreen',
    tsx: 'stores/MultiStoreDashboardScreen.tsx',
    css: ['stores/MultiStoreDashboardScreen.css'],
    externalClasses: [
      'multi-store-view-toggle',
      'multi-store-dashboard-topology-view',
    ],
  },
  {
    name: 'TerminalStatusPanel',
    tsx: 'stores/TerminalStatusPanel.tsx',
    css: ['stores/TerminalStatusPanel.css'],
  },

  // ── Tables ────────────────────────────────────────────
  {
    name: 'TableManagementScreen',
    tsx: 'tables/TableManagementScreen.tsx',
    css: ['tables/TableManagementScreen.css'],
    dynamicClassPrefixes: ['tables-table--'],
  },

  // ── Tax ───────────────────────────────────────────────
  {
    name: 'TaxConfigurationScreen',
    tsx: 'tax/TaxConfigurationScreen.tsx',
    css: ['tax/TaxConfigurationScreen.css'],
  },

  // ── Terminals ─────────────────────────────────────────
  {
    name: 'TerminalManagementScreen',
    tsx: 'terminals/TerminalManagementScreen.tsx',
    css: ['terminals/TerminalManagementScreen.css'],
  },

  // ── Workspaces ────────────────────────────────────────
  {
    name: 'WorkspaceHome',
    tsx: 'workspaces/WorkspaceHome.tsx',
    css: ['workspaces/WorkspaceHome.css'],
    dynamicClassPrefixes: ['ws-color-', 'role-badge--'],
    externalClasses: [
      'workspace-home-user',
      'workspace-card--exiting',
      'workspace-card--active',
      'workspace-card-ripple',
    ],
  },

  // ── Kiosk ─────────────────────────────────────────────
  {
    name: 'KioskScreen',
    tsx: 'kiosk/KioskScreen.tsx',
    css: ['kiosk/KioskScreen.css'],
  },

  // ── Sales (those with a single companion CSS) ─────────
  {
    name: 'SalesDashboardScreen',
    tsx: 'sales/SalesDashboardScreen.tsx',
    css: ['sales/SalesDashboardScreen.css'],
  },
  {
    name: 'SalesHistoryScreen',
    tsx: 'sales/SalesHistoryScreen.tsx',
    css: ['sales/SalesHistoryScreen.css'],
  },
  {
    name: 'VoidOrdersScreen',
    tsx: 'sales/VoidOrdersScreen.tsx',
    css: ['sales/VoidOrdersScreen.css'],
  },
  {
    name: 'EodReportScreen',
    tsx: 'sales/EodReportScreen.tsx',
    css: ['sales/EodReportScreen.css'],
  },
  {
    name: 'RefundModal',
    tsx: 'sales/RefundModal.tsx',
    css: ['sales/RefundModal.css'],
  },
  {
    name: 'PriceOverrideModal',
    tsx: 'sales/PriceOverrideModal.tsx',
    css: ['sales/PriceOverrideModal.css'],
    dynamicClassPrefixes: ['price-override-pin-dot--'],
  },

  // ── Reports ───────────────────────────────────────────
  {
    name: 'DashboardScreen',
    tsx: 'reports/DashboardScreen.tsx',
    css: ['reports/DashboardScreen.css'],
  },
  {
    name: 'InventoryReportScreen',
    tsx: 'reports/InventoryReportScreen.tsx',
    css: ['reports/InventoryReportScreen.css'],
  },
  {
    name: 'SalesReportScreen',
    tsx: 'reports/SalesReportScreen.tsx',
    css: ['reports/SalesReportScreen.css'],
  },

  // ── Gift Cards ─────────────────────────────────────────
  {
    name: 'GiftCardsScreen',
    tsx: 'gift-cards/GiftCardsScreen.tsx',
    css: ['gift-cards/GiftCardsScreen.css'],
    dynamicClassPrefixes: ['gift-card-status--', 'gift-card-txn-type--'],
    externalClasses: [
      'gift-cards-modal-overlay',
      'gift-cards-modal-overlay--exiting',
      'gift-cards-modal',
      'gift-cards-modal--exiting',
      'gift-cards-modal-title',
      'gift-cards-modal-form',
      'gift-cards-modal-field',
      'gift-cards-modal-label',
      'gift-cards-modal-input',
      'gift-cards-modal-error',
      'gift-cards-modal-actions',
    ],
  },

  // ── Stock Counting ─────────────────────────────────────
  {
    name: 'StockCountsScreen',
    tsx: 'inventory/StockCountsScreen.tsx',
    css: ['inventory/StockCountsScreen.css'],
    dynamicClassPrefixes: ['sc-badge--'],
    externalClasses: ['sc-card-type', 'sc-card-date', 'sc-badge'],
  },
  {
    name: 'StockCountDetail',
    tsx: 'inventory/StockCountDetail.tsx',
    css: ['inventory/StockCountDetail.css'],
    dynamicClassPrefixes: ['sc-badge--', 'sc-add-line-item--', 'sc-diff-'],
    knownDynamicFragments: [
      // String-interpolated fragments in the skeleton table header that
      // the static class-name parser falsely extracts as CSS classes.
      // These are template-literal substrings like 'sc-lines-col-' + suffix.
      'sc-lines-col-',
      'sku',
      'name',
      'expected',
      'counted',
      'diff',
    ],
  },
  {
    name: 'StockCountForm',
    tsx: 'inventory/StockCountForm.tsx',
    css: ['inventory/StockCountForm.css'],
    dynamicClassPrefixes: ['sc-type-btn--'],
  },
  {
    name: 'StockCountHistory',
    tsx: 'inventory/StockCountHistory.tsx',
    css: ['inventory/StockCountHistory.css'],
    dynamicClassPrefixes: ['sc-hist-item--'],
  },

  // ── Stock Transfers ────────────────────────────────────
  {
    name: 'StockTransfersScreen',
    tsx: 'stock-transfers/StockTransfersScreen.tsx',
    css: ['stock-transfers/StockTransfersScreen.css'],
    dynamicClassPrefixes: ['stock-transfers-badge--'],
    externalClasses: ['stock-transfers-detail'],
  },

  // ── Purchasing ─────────────────────────────────────────
  {
    name: 'SuppliersScreen',
    tsx: 'purchasing/SuppliersScreen.tsx',
    css: ['purchasing/SuppliersScreen.css'],
    dynamicClassPrefixes: ['suppliers-badge--'],
  },
  {
    name: 'PurchaseOrdersScreen',
    tsx: 'purchasing/PurchaseOrdersScreen.tsx',
    css: ['purchasing/PurchaseOrdersScreen.css'],
    dynamicClassPrefixes: ['po-status--'],
  },
  {
    name: 'PurchaseOrderForm',
    tsx: 'purchasing/PurchaseOrderForm.tsx',
    css: ['purchasing/PurchaseOrderForm.css'],
  },

  // ── Restaurant ─────────────────────────────────────────
  {
    name: 'RestaurantMenu',
    tsx: 'restaurant/RestaurantMenu.tsx',
    css: ['restaurant/RestaurantMenu.css'],
    dynamicClassPrefixes: ['restaurant-hamburger-item--', 'restaurant-card--'],
    externalClasses: ['restaurant-card', 'restaurant-pill-dot'],
  },

  // ── Appearance Settings ────────────────────────────────
  {
    name: 'AppearanceSettings',
    tsx: 'settings/AppearanceSettings.tsx',
    css: ['settings/AppearanceSettings.css', 'settings/SettingsPage.css'],
    dynamicClassPrefixes: [
      'settings-',
      'tooltip-content',
      'feature-toggle',
      'data-mgmt',
      'staff-mgmt',
      'terminal-mgmt',
      'multi-store-dashboard',
      'audit-log',
      'offline-queue-screen',
      'shift-mgmt',
      'tax-config',
      'exchange-rate-config',
      'promo-mgmt',
    ],
    knownDynamicFragments: [
      // Card component classes (defined in frontend/themes/components.css)
      // that are used inline in AppearanceSettings.tsx but not present
      // in the screen's own CSS files.
      'card--padding-md',
      'card--shadow-sm',
      'card-header',
    ],
    externalClasses: [
      'card',
      'tab-list',
      'collapsed',
      'mobile-open',
      'visible',
    ],
  },
];

// ── Tests ─────────────────────────────────────────────────────────

describe.each(SCREENS)(
  'CSS class integrity — $name',
  ({ name, tsx, css, dynamicClassPrefixes, externalClasses, knownDynamicFragments }: ScreenEntry) => {
    const tsxPath = path.join(FEATURES_DIR, tsx);
    const tsxContent = fs.readFileSync(tsxPath, 'utf8');
    const used = extractUsedClassNames(tsxContent);

    // Build reverse map: className -> [file1, file2, ...]
    const fileIndex = new Map<string, string[]>();

    // Track unique files to avoid counting the same path twice
    // when the same class appears in the same file via compound selectors.
    const cssPaths = css.map((c) => path.join(FEATURES_DIR, c));

    for (const cssPath of cssPaths) {
      const content = fs.readFileSync(cssPath, 'utf8');
      for (const cls of extractClassSelectors(content)) {
        if (!fileIndex.has(cls)) {
          fileIndex.set(cls, []);
        }
        fileIndex.get(cls)!.push(cssPath);
      }
    }

    it(`every className used in ${name} has a CSS rule defined`, () => {
      const fragments = new Set(knownDynamicFragments ?? []);
      const missing: string[] = [];
      for (const cls of used) {
        if (!fileIndex.has(cls) && !fragments.has(cls)) {
          missing.push(cls);
        }
      }
      expect(
        missing,
        `${name}: className(s) used but not defined: ${missing.join(', ')}`,
      ).toEqual([]);
    });

    it(`no className is defined in more than one CSS file for ${name}`, () => {
      const duplicates: string[] = [];
      for (const [cls, files] of fileIndex) {
        // Only flag if the class appears in multiple unique files
        const uniqueFiles = [...new Set(files)];
        if (uniqueFiles.length > 1 && used.has(cls)) {
          duplicates.push(
            `${cls} -> ${uniqueFiles.map((f) => path.basename(f)).join(', ')}`,
          );
        }
      }
      expect(
        duplicates,
        `${name}: className(s) duplicated across files:\n${duplicates.join('\n')}`,
      ).toEqual([]);
    });

    it(`every className defined in CSS is reachable from ${name} (no dead classes)`, () => {
      const prefixes = dynamicClassPrefixes ?? [];
      const external = new Set(externalClasses ?? []);
      const dead: string[] = [];
      for (const [cls] of fileIndex) {
        if (!used.has(cls) && !external.has(cls) && !prefixes.some((p) => cls.startsWith(p))) {
          dead.push(cls);
        }
      }
      // Soft assertion — logs a warning rather than hard-failing,
      // because some classes may be shared with other components.
      if (dead.length > 0) {
        console.warn(
          `[WARN] ${name}: className(s) defined in CSS but never referenced ` +
            `(consider removing if unused elsewhere):\n  ${dead.join('\n  ')}`,
        );
        expect.soft(dead, `Dead classes: ${dead.join(', ')}`).toEqual([]);
      }
    });
  },
);
