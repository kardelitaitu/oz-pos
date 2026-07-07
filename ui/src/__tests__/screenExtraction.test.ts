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
    dynamicClassPrefixes: ['kds-column--'],
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
    externalClasses: ['card', 'tab-list'],
  },
  {
    name: 'DataManagementScreen',
    tsx: 'settings/DataManagementScreen.tsx',
    css: ['settings/DataManagementScreen.css'],
    dynamicClassPrefixes: ['data-mgmt-toast--'],
  },
  {
    name: 'FeatureToggleScreen',
    tsx: 'settings/FeatureToggleScreen.tsx',
    css: ['settings/FeatureToggleScreen.css'],
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
      'gift-cards-modal',
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
    externalClasses: [
      'settings-page',
      'settings-title',
      'card',
      'tab-list',
      'settings-toggle',
      'settings-field',
      'settings-select',
      'settings-hint',
      'settings-size-controls',
      'settings-size-btn',
      'settings-size-value',
    ],
  },
];

// ── Tests ─────────────────────────────────────────────────────────

describe.each(SCREENS)(
  'CSS class integrity — $name',
  ({ name, tsx, css, dynamicClassPrefixes, externalClasses }: ScreenEntry) => {
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
      const missing: string[] = [];
      for (const cls of used) {
        if (!fileIndex.has(cls)) {
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
