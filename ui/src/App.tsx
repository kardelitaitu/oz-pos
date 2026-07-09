import { ThemeProvider } from '@/frontend/shell/ThemeProvider';
import { BrandProvider } from '@/contexts/BrandContext';
import { AuthProvider } from '@/contexts/AuthContext';
import { ToastProvider } from '@/frontend/shared/Toast';
import { LocaleProvider } from './i18n/LocaleContext';
import AppShell from '@/frontend/shell/AppShell';
import { registerPage } from '@/platform/ui/page-registry';
import { registerNavItem } from '@/platform/ui/menu-registry';

// ── Register all feature pages ──────────────────────────────────────
//
// Each feature registers its screen component and nav item with the
// platform registries so the AppShell can render them dynamically.

import PosScreen from '@/features/sales/PosScreen';
import SalesHistoryScreen from '@/features/sales/SalesHistoryScreen';
import SalesDashboardScreen from '@/features/sales/SalesDashboardScreen';
import EodReportScreen from '@/features/sales/EodReportScreen';
import VoidOrdersScreen from '@/features/sales/VoidOrdersScreen';
import TaxConfigurationScreen from '@/features/tax/TaxConfigurationScreen';
import ExchangeRateScreen from '@/features/currency/ExchangeRateScreen';
import ProductLookupScreen from '@/features/products/ProductLookupScreen';
import ProductManagementScreen from '@/features/products/ProductManagementScreen';
import BundleManagementScreen from '@/features/products/BundleManagementScreen';
import CategoryManagementScreen from '@/features/categories/CategoryManagementScreen';
import InventoryAdjustmentScreen from '@/features/inventory/InventoryAdjustmentScreen';
import SettingsPage from '@/features/settings/SettingsPage';
import FeatureToggleScreen from '@/features/settings/FeatureToggleScreen';
import DataManagementScreen from '@/features/settings/DataManagementScreen';
import CustomerManagementScreen from '@/features/customers/CustomerManagementScreen';
import GiftCardsScreen from '@/features/gift-cards/GiftCardsScreen';
import LoyaltyManagementScreen from '@/features/loyalty/LoyaltyManagementScreen';
import StaffManagementScreen from '@/features/staff/StaffManagementScreen';
import TerminalManagementScreen from '@/features/terminals/TerminalManagementScreen';
import { MultiStoreDashboardScreen } from '@/features/stores';
import AuditLogScreen from '@/features/audit/AuditLogScreen';
import OfflineQueueScreen from '@/features/offline/OfflineQueueScreen';
import ShiftManagementScreen from '@/features/shifts/ShiftManagementScreen';
import PromotionManagementScreen from '@/features/promotions/PromotionManagementScreen';
import DashboardScreen from '@/features/reports/DashboardScreen';
import SalesReportScreen from '@/features/reports/SalesReportScreen';
import InventoryReportScreen from '@/features/reports/InventoryReportScreen';
import DesignSystem from '@/features/design/DesignSystem';
import KdsScreen from '@/features/kds/KdsScreen';
import KioskScreen from '@/features/kiosk/KioskScreen';
import TableManagementScreen from '@/features/tables/TableManagementScreen';
import SuppliersScreen from '@/features/purchasing/SuppliersScreen';
import PurchaseOrdersScreen from '@/features/purchasing/PurchaseOrdersScreen';
import StockCountsFlow from '@/features/inventory/StockCountsFlow';
import StockTransfersScreen from '@/features/stock-transfers/StockTransfersScreen';
import { registerSalesWidgets } from '@/features/sales/widgets';

// ── Register dashboard widgets ────────────────────────────────────
registerSalesWidgets();

// ── SVG icon factory ────────────────────────────────────────────────
// Accepts a path `d` string and optional children (ReactNode) for
// extra SVG elements beyond the first <path>.
import type { ReactNode } from 'react';

function icon(path: string, ...children: ReactNode[]) {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d={path} />
      {children}
    </svg>
  );
}

// ── Register all pages and nav items ────────────────────────────────

registerPage({ route: 'sales', component: PosScreen, label: 'POS Terminal', feature: 'simple-retail' });
registerNavItem({ route: 'sales', label: 'POS Terminal', feature: 'simple-retail', i18nKey: 'nav-pos-terminal', section: 'operations', icon: icon('M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z', <polyline points="3.29 7 12 12 20.71 7" />) });

registerPage({ route: 'kds', component: KdsScreen, label: 'KDS', feature: 'kds' });
registerNavItem({ route: 'kds', label: 'KDS', feature: 'kds', i18nKey: 'nav-kds', section: 'operations',
  icon: icon('M9 5H7a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V7a2 2 0 0 0-2-2h-2M9 5a2 2 0 0 1 2-2h2a2 2 0 0 1 2 2M9 5h6') });

registerPage({ route: 'products', component: ProductLookupScreen, label: 'Products' });
registerNavItem({ route: 'products', label: 'Products', i18nKey: 'nav-products', section: 'products', icon: icon('M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z') });

registerPage({ route: 'inventory', component: ProductManagementScreen, label: 'Inventory', requiredRole: 'manager' });
registerNavItem({ route: 'inventory', label: 'Inventory', requiredRole: 'manager', i18nKey: 'nav-inventory', section: 'products', icon: icon('M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2', <rect x="8" y="2" width="8" height="4" rx="1" ry="1" />, <line x1="8" y1="12" x2="16" y2="12" />, <line x1="8" y1="16" x2="14" y2="16" />) });

registerPage({ route: 'inventory-adjustment', component: InventoryAdjustmentScreen, label: 'Stock Adjust', requiredRole: 'manager' });
registerNavItem({ route: 'inventory-adjustment', label: 'Stock Adjust', requiredRole: 'manager', i18nKey: 'nav-stock-adjust', section: 'products', icon: icon('M12 5v14M5 12h14', <line x1="12" y1="5" x2="12" y2="19" />, <line x1="5" y1="12" x2="19" y2="12" />) });

registerPage({ route: 'sales-history', component: SalesHistoryScreen, label: 'Sales History', feature: 'simple-retail' });
registerNavItem({ route: 'sales-history', label: 'Sales History', feature: 'simple-retail', i18nKey: 'nav-sales-history', section: 'sales', icon: icon('M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z', <polyline points="14 2 14 8 20 8" />, <line x1="16" y1="13" x2="8" y2="13" />, <line x1="16" y1="17" x2="8" y2="17" />) });

registerPage({ route: 'sales-dashboard', component: SalesDashboardScreen, label: 'Dashboard', feature: 'simple-retail' });
registerNavItem({ route: 'sales-dashboard', label: 'Dashboard', feature: 'simple-retail', i18nKey: 'nav-dashboard', section: 'sales', icon: icon('M3 3h7v7H3zM14 3h7v7h-7zM14 14h7v7h-7zM3 14h7v7H3z') });

registerPage({ route: 'eod-report', component: EodReportScreen, label: 'EOD Report', requiredRole: 'manager' });
registerNavItem({ route: 'eod-report', label: 'EOD Report', requiredRole: 'manager', i18nKey: 'nav-eod-report', section: 'sales', icon: icon('M21.21 15.89A10 10 0 1 1 8 2.83', <path d="M22 12A10 10 0 0 0 12 2v10z" />) });

registerPage({ route: 'orders', component: VoidOrdersScreen, label: 'Orders', feature: 'simple-retail', requiredRole: 'manager' });
registerNavItem({ route: 'orders', label: 'Orders', feature: 'simple-retail', requiredRole: 'manager', i18nKey: 'nav-orders', section: 'sales', icon: icon('M9 5H7a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V7a2 2 0 0 0-2-2h-2', <rect x="9" y="3" width="6" height="4" rx="1" />, <path d="M9 14l2 2 4-4" />) });

registerPage({ route: 'tax-config', component: TaxConfigurationScreen, label: 'Tax Rates', feature: 'tax-engine', requiredRole: 'manager' });
registerNavItem({ route: 'tax-config', label: 'Tax Rates', feature: 'tax-engine', requiredRole: 'manager', i18nKey: 'nav-tax-rates', section: 'finance', icon: icon('M12 1v22M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6', <line x1="12" y1="1" x2="12" y2="23" />) });

registerPage({ route: 'exchange-rates', component: ExchangeRateScreen, label: 'Exchange Rates', requiredRole: 'manager' });
registerNavItem({ route: 'exchange-rates', label: 'Exchange Rates', requiredRole: 'manager', i18nKey: 'nav-exchange-rates', section: 'finance', icon: icon('M12 1v22M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6', <line x1="12" y1="1" x2="12" y2="23" />) });

registerPage({ route: 'categories', component: CategoryManagementScreen, label: 'Categories', feature: 'categories-enabled', requiredRole: 'manager' });
registerNavItem({ route: 'categories', label: 'Categories', feature: 'categories-enabled', requiredRole: 'manager', i18nKey: 'nav-categories', section: 'products', icon: icon('M4 6h16M4 12h16M4 18h10') });

registerPage({ route: 'customers', component: CustomerManagementScreen, label: 'Customers' });
registerNavItem({ route: 'customers', label: 'Customers', i18nKey: 'nav-customers', section: 'customers', icon: icon('M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2', <circle cx="9" cy="7" r="4" />) });

registerPage({ route: 'gift-cards', component: GiftCardsScreen, label: 'Gift Cards', feature: 'gift-cards', requiredRole: 'manager' });
registerNavItem({ route: 'gift-cards', label: 'Gift Cards', feature: 'gift-cards', requiredRole: 'manager', i18nKey: 'nav-gift-cards', section: 'customers',
  icon: icon('M20 12H4M12 4v16M4 8h16M8 4v16') });

registerPage({ route: 'loyalty', component: LoyaltyManagementScreen, label: 'Loyalty', requiredRole: 'manager' });
registerNavItem({ route: 'loyalty', label: 'Loyalty', requiredRole: 'manager', i18nKey: 'nav-loyalty', section: 'customers',
  icon: icon('M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z') });

registerPage({ route: 'staff', component: StaffManagementScreen, label: 'Staff', requiredRole: 'manager' });
registerNavItem({ route: 'staff', label: 'Staff', requiredRole: 'manager', i18nKey: 'nav-staff', section: 'management', icon: icon('M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2', <circle cx="9" cy="7" r="4" />, <path d="M23 21v-2a4 4 0 0 0-3-3.87" />, <path d="M16 3.13a4 4 0 0 1 0 7.75" />) });

registerPage({ route: 'terminals', component: TerminalManagementScreen, label: 'Terminals', requiredRole: 'manager' });
registerNavItem({ route: 'terminals', label: 'Terminals', requiredRole: 'manager', i18nKey: 'nav-terminals', section: 'management', icon: icon('M2 3h20v14H2z', <line x1="8" y1="21" x2="16" y2="21" />, <line x1="12" y1="17" x2="12" y2="21" />, <path d="M7 7l3 3-3 3" />) });

registerPage({ route: 'stores', component: MultiStoreDashboardScreen, label: 'Stores', feature: 'multi-store', requiredRole: 'manager' });
registerNavItem({ route: 'stores', label: 'Stores', feature: 'multi-store', requiredRole: 'manager', i18nKey: 'nav-stores', section: 'management', icon: icon('M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z', <polyline points="9 22 9 12 15 12 15 22" />) });

registerPage({ route: 'features', component: FeatureToggleScreen, label: 'Features', requiredRole: 'owner' });
registerNavItem({ route: 'features', label: 'Features', requiredRole: 'owner', i18nKey: 'nav-features', section: 'management', icon: icon('M13 2 3 14h9l-1 8 10-12h-9z') });

registerPage({ route: 'data-management', component: DataManagementScreen, label: 'Data', requiredRole: 'owner' });
registerNavItem({ route: 'data-management', label: 'Data', requiredRole: 'owner', i18nKey: 'nav-data', section: 'management', icon: icon('M12 5c-5 0-9 1.34-9 3s4 3 9 3 9-1.34 9-3-4-3-9-3z', <ellipse cx="12" cy="5" rx="9" ry="3" />, <path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3" />, <path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5" />) });

registerPage({ route: 'audit-log', component: AuditLogScreen, label: 'Audit Log', requiredRole: 'manager' });
registerNavItem({ route: 'audit-log', label: 'Audit Log', requiredRole: 'manager', i18nKey: 'nav-audit-log', section: 'management', icon: icon('M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z', <polyline points="14 2 14 8 20 8" />, <line x1="16" y1="13" x2="8" y2="13" />, <line x1="16" y1="17" x2="8" y2="17" />) });

registerPage({ route: 'offline-queue', component: OfflineQueueScreen, label: 'Offline Queue', requiredRole: 'manager' });
registerNavItem({ route: 'offline-queue', label: 'Offline Queue', requiredRole: 'manager', i18nKey: 'nav-offline-queue', section: 'management', icon: icon('M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z') });

registerPage({ route: 'shifts', component: ShiftManagementScreen, label: 'Shifts', requiredRole: 'manager' });
registerNavItem({ route: 'shifts', label: 'Shifts', requiredRole: 'manager', i18nKey: 'nav-shifts', section: 'management', icon: icon('M12 1v22M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6', <line x1="12" y1="1" x2="12" y2="23" />) });

registerPage({ route: 'bundles', component: BundleManagementScreen, label: 'Bundles', requiredRole: 'manager' });
registerNavItem({ route: 'bundles', label: 'Bundles', requiredRole: 'manager', i18nKey: 'nav-bundles', section: 'products',
  icon: icon('M16 11V7a4 4 0 0 0-8 0v4M5 9h14l1 12H4L5 9z') });

registerPage({ route: 'settings', component: SettingsPage, label: 'Settings', requiredRole: 'manager' });
registerNavItem({ route: 'settings', label: 'Settings', requiredRole: 'manager', i18nKey: 'nav-settings', section: 'settings', icon: icon('M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42') });

registerPage({ route: 'dashboard', component: DashboardScreen, label: 'Dashboard' });
registerNavItem({ route: 'dashboard', label: 'Dashboard', i18nKey: 'nav-dashboard-report', section: 'reports',
  icon: icon('M3 13h8V3H3v10zm0 8h8v-6H3v6zm10 0h8V11h-8v10zm0-18v6h8V3h-8z') });

registerPage({ route: 'reports', component: SalesReportScreen, label: 'Sales Report', requiredRole: 'manager' });
registerNavItem({ route: 'reports', label: 'Sales Report', requiredRole: 'manager', i18nKey: 'nav-sales-report', section: 'reports',
  icon: icon('M21.21 15.89A10 10 0 1 1 8 2.83M22 12A10 10 0 0 0 12 2v10z') });

registerPage({ route: 'inventory-report', component: InventoryReportScreen, label: 'Inventory Report', requiredRole: 'manager' });
registerNavItem({ route: 'inventory-report', label: 'Inventory Report', requiredRole: 'manager', i18nKey: 'nav-inventory-report', section: 'reports',
  icon: icon('M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z') });

registerPage({ route: 'design', component: DesignSystem, label: 'Design System' });
registerNavItem({ route: 'design', label: 'Design System', i18nKey: 'nav-design-system', section: 'dev', icon: icon('M12 12a3 3 0 1 0 0-6 3 3 0 0 0 0 6z', <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />) });

registerPage({ route: 'kiosk', component: KioskScreen, label: 'Kiosk', feature: 'self-service-kiosk', fullscreen: true });
registerNavItem({ route: 'kiosk', label: 'Kiosk', feature: 'self-service-kiosk', i18nKey: 'nav-kiosk', section: 'operations',
  icon: icon('M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z') });

registerPage({ route: 'tables', component: TableManagementScreen, label: 'Tables', feature: 'tables' });
registerNavItem({ route: 'tables', label: 'Tables', feature: 'tables', i18nKey: 'nav-tables', section: 'operations',
  icon: icon('M3 3h7v7H3V3zm11 0h7v7h-7V3zM3 14h7v7H3v-7zm11 0h7v7h-7v-7z') });

registerPage({ route: 'promotions', component: PromotionManagementScreen, label: 'Promotions', requiredRole: 'manager' });
registerNavItem({ route: 'promotions', label: 'Promotions', requiredRole: 'manager', i18nKey: 'nav-promotions', section: 'finance',
  icon: icon('M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z') });

registerPage({ route: 'suppliers', component: SuppliersScreen, label: 'Suppliers', feature: 'purchase-orders' });
registerNavItem({ route: 'suppliers', label: 'Suppliers', feature: 'purchase-orders', i18nKey: 'nav-suppliers', section: 'inventory',
  icon: icon('M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2', <circle cx="9" cy="7" r="4" />, <path d="M22 21v-2a4 4 0 0 0-3-3.87" />, <path d="M16 3.13a4 4 0 0 1 0 7.75" />) });

registerPage({ route: 'purchase-orders', component: PurchaseOrdersScreen, label: 'Purchase Orders', feature: 'purchase-orders' });
registerNavItem({ route: 'purchase-orders', label: 'Purchase Orders', feature: 'purchase-orders', i18nKey: 'nav-purchase-orders', section: 'inventory',
  icon: icon('M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z', <polyline points="14 2 14 8 20 8" />, <line x1="16" y1="13" x2="8" y2="13" />, <line x1="16" y1="17" x2="8" y2="17" />, <polyline points="10 9 9 9 8 9" />) });

registerPage({ route: 'stock-counts', component: StockCountsFlow, label: 'Stock Counts', feature: 'stock-counting' });
registerNavItem({ route: 'stock-counts', label: 'Stock Counts', feature: 'stock-counting', i18nKey: 'nav-stock-counts', section: 'inventory',
  icon: icon('M9 5H7a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V7a2 2 0 0 0-2-2h-2M9 5a2 2 0 0 1 2-2h2a2 2 0 0 1 2 2', <path d="M9 14l2 2 4-4" />) });

registerPage({ route: 'stock-transfers', component: StockTransfersScreen, label: 'Stock Transfers', feature: 'stock-transfers' });
registerNavItem({ route: 'stock-transfers', label: 'Stock Transfers', feature: 'stock-transfers', i18nKey: 'nav-stock-transfers', section: 'inventory',
  icon: icon('M5 12h14M12 5l7 7-7 7') });

// ── Root App component ──────────────────────────────────────────────

/**
 * Root app component. Provides theme, auth, and toast contexts,
 * then delegates to AppShell which handles routing and layout.
 *
 * Feature pages are registered above so the AppShell can render
 * them dynamically from the page-registry instead of a hardcoded switch.
 */
import ErrorBoundary from '@/components/ErrorBoundary';
import { WorkspaceProvider } from '@/contexts/WorkspaceContext';

export default function App() {
  return (
    <ErrorBoundary>
      <LocaleProvider>
        <BrandProvider>
        <ThemeProvider>
          <AuthProvider>
            <ToastProvider>
              <WorkspaceProvider>
                <AppShell />
              </WorkspaceProvider>
            </ToastProvider>
          </AuthProvider>
        </ThemeProvider>
        </BrandProvider>
      </LocaleProvider>
    </ErrorBoundary>
  );
}
