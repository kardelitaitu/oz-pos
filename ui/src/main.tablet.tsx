import React from 'react';
import ReactDOM from 'react-dom/client';
import { LocalizationProvider } from '@fluent/react';
import { createEnUsLocalization } from './locales';
import { ThemeProvider } from '@/frontend/shell/ThemeProvider';
import { AuthProvider } from '@/contexts/AuthContext';
import { WorkspaceProvider } from '@/contexts/WorkspaceContext';
import { ToastProvider } from '@/frontend/shared/Toast';
import TabletAppShell from '@/frontend/shell/tablet/TabletAppShell';
import { registerPage } from '@/platform/ui/page-registry';
import { registerNavItem } from '@/platform/ui/menu-registry';
import { registerSalesWidgets } from '@/features/sales/widgets';
import './frontend/themes/reset.css';
import './frontend/themes/tokens.css';
import './frontend/themes/components.css';
import './frontend/themes/responsive.css';

// ── Register all feature pages (same as desktop App.tsx) ──────────

import PosScreen from '@/features/sales/PosScreen';
import SalesHistoryScreen from '@/features/sales/SalesHistoryScreen';
import SalesDashboardScreen from '@/features/sales/SalesDashboardScreen';
import EodReportScreen from '@/features/sales/EodReportScreen';
import VoidOrdersScreen from '@/features/sales/VoidOrdersScreen';
import TaxConfigurationScreen from '@/features/tax/TaxConfigurationScreen';
import ExchangeRateScreen from '@/features/currency/ExchangeRateScreen';
import ProductLookupScreen from '@/features/products/ProductLookupScreen';
import ProductManagementScreen from '@/features/products/ProductManagementScreen';
import CategoryManagementScreen from '@/features/categories/CategoryManagementScreen';
import InventoryAdjustmentScreen from '@/features/inventory/InventoryAdjustmentScreen';
import SettingsPage from '@/features/settings/SettingsPage';
import FeatureToggleScreen from '@/features/settings/FeatureToggleScreen';
import DataManagementScreen from '@/features/settings/DataManagementScreen';
import CustomerManagementScreen from '@/features/customers/CustomerManagementScreen';
import StaffManagementScreen from '@/features/staff/StaffManagementScreen';
import TerminalManagementScreen from '@/features/terminals/TerminalManagementScreen';
import { MultiStoreDashboardScreen } from '@/features/stores';
import AuditLogScreen from '@/features/audit/AuditLogScreen';
import OfflineQueueScreen from '@/features/offline/OfflineQueueScreen';
import _DesignSystem from '@/features/design/DesignSystem';
import MenuEngineeringScreen from '@/features/reports/MenuEngineeringScreen';
import { Children, type ReactNode } from 'react';

// ── SVG icon factory ─────────────────────────────────────────────
function icon(path: string, ...children: ReactNode[]) {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"
         strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d={path} />
      {Children.toArray(children)}
    </svg>
  );
}

// ── Register dashboard widgets ───────────────────────────────────
registerSalesWidgets();

// ── Register all pages and nav items ─────────────────────────────
registerPage({ route: 'pos', component: PosScreen, label: 'POS Terminal', feature: 'simple-retail' });
registerNavItem({ route: 'pos', label: 'POS', feature: 'simple-retail', i18nKey: 'nav-pos', icon: icon('M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z', <polyline points="3.29 7 12 12 20.71 7" />) });

registerPage({ route: 'products', component: ProductLookupScreen, label: 'Products' });
registerNavItem({ route: 'products', label: 'Products', i18nKey: 'nav-products', icon: icon('M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z') });

registerPage({ route: 'inventory', component: ProductManagementScreen, label: 'Inventory', requiredRole: 'manager' });
registerNavItem({ route: 'inventory', label: 'Stock', requiredRole: 'manager', i18nKey: 'nav-stock', icon: icon('M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2', <rect x="8" y="2" width="8" height="4" rx="1" ry="1" />) });

registerPage({ route: 'sales-history', component: SalesHistoryScreen, label: 'Sales History', feature: 'simple-retail' });
registerNavItem({ route: 'sales-history', label: 'History', feature: 'simple-retail', i18nKey: 'nav-history', icon: icon('M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z', <polyline points="14 2 14 8 20 8" />) });

registerPage({ route: 'sales-dashboard', component: SalesDashboardScreen, label: 'Dashboard', feature: 'simple-retail' });
registerNavItem({ route: 'sales-dashboard', label: 'Reports', feature: 'simple-retail', i18nKey: 'nav-reports', icon: icon('M3 3h7v7H3zM14 3h7v7h-7zM14 14h7v7h-7zM3 14h7v7H3z') });

registerPage({ route: 'customers', component: CustomerManagementScreen, label: 'Customers' });
registerNavItem({ route: 'customers', label: 'Customers', i18nKey: 'nav-customers', icon: icon('M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2', <circle cx="9" cy="7" r="4" />) });

registerPage({ route: 'settings', component: SettingsPage, label: 'Settings', requiredRole: 'manager' });
registerNavItem({ route: 'settings', label: 'Settings', requiredRole: 'manager', i18nKey: 'nav-settings', icon: icon('M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42') });

// Also register pages without nav items (accessible from settings/management screens)
registerPage({ route: 'eod-report', component: EodReportScreen, label: 'EOD Report', requiredRole: 'manager' });
registerPage({ route: 'orders', component: VoidOrdersScreen, label: 'Orders', feature: 'simple-retail', requiredRole: 'manager' });
registerPage({ route: 'tax-config', component: TaxConfigurationScreen, label: 'Tax Rates', feature: 'tax-engine', requiredRole: 'manager' });
registerPage({ route: 'exchange-rates', component: ExchangeRateScreen, label: 'Exchange Rates', requiredRole: 'manager' });
registerPage({ route: 'categories', component: CategoryManagementScreen, label: 'Categories', feature: 'categories-enabled', requiredRole: 'manager' });
registerPage({ route: 'inventory-adjustment', component: InventoryAdjustmentScreen, label: 'Stock Adjust', requiredRole: 'manager' });
registerPage({ route: 'staff', component: StaffManagementScreen, label: 'Staff', requiredRole: 'manager' });
registerPage({ route: 'terminals', component: TerminalManagementScreen, label: 'Terminals', requiredRole: 'manager' });
registerPage({ route: 'stores', component: MultiStoreDashboardScreen, label: 'Stores', feature: 'multi-store', requiredRole: 'manager' });
registerPage({ route: 'features', component: FeatureToggleScreen, label: 'Features', requiredRole: 'owner' });
registerPage({ route: 'data-management', component: DataManagementScreen, label: 'Data', requiredRole: 'owner' });
registerPage({ route: 'audit-log', component: AuditLogScreen, label: 'Audit Log', requiredRole: 'manager' });
registerPage({ route: 'offline-queue', component: OfflineQueueScreen, label: 'Offline Queue', requiredRole: 'manager' });

registerPage({ route: 'menu-engineering', component: MenuEngineeringScreen, label: 'Menu Engineering', feature: 'restaurant', requiredRole: 'manager' });

// ── Render ───────────────────────────────────────────────────────
const l10n = createEnUsLocalization();

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <LocalizationProvider l10n={l10n}>
      <ThemeProvider>
        <AuthProvider>
          <WorkspaceProvider>
            <ToastProvider>
              <TabletAppShell />
            </ToastProvider>
          </WorkspaceProvider>
        </AuthProvider>
      </ThemeProvider>
    </LocalizationProvider>
  </React.StrictMode>,
);
