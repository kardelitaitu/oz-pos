import { useState, useEffect, useCallback } from 'react';
import { ThemeProvider } from '@/components/ThemeProvider';
import { AuthProvider } from '@/contexts/AuthContext';
import { ToastProvider } from '@/hooks/useToast';
import { useAuth } from '@/contexts/AuthContext';
import StaffLoginScreen from '@/features/auth/StaffLoginScreen';
import StaffManagementScreen from '@/features/staff/StaffManagementScreen';
import TerminalManagementScreen from '@/features/terminals/TerminalManagementScreen';
import CustomerManagementScreen from '@/features/customers/CustomerManagementScreen';
import AppLayout from '@/components/AppLayout';
import SetupWizard from '@/features/setup/SetupWizard';
import DesignSystem from '@/features/design/DesignSystem';
import PosScreen from '@/features/sales/PosScreen';
import SalesHistoryScreen from '@/features/sales/SalesHistoryScreen';
import SalesDashboardScreen from '@/features/sales/SalesDashboardScreen';
import VoidOrdersScreen from '@/features/sales/VoidOrdersScreen';
import TaxConfigurationScreen from '@/features/tax/TaxConfigurationScreen';
import ExchangeRateScreen from '@/features/currency/ExchangeRateScreen';
import ProductLookupScreen from '@/features/products/ProductLookupScreen';
import ProductManagementScreen from '@/features/products/ProductManagementScreen';
import CategoryManagementScreen from '@/features/categories/CategoryManagementScreen';
import SettingsPage from '@/features/settings/SettingsPage';
import FeatureToggleScreen from '@/features/settings/FeatureToggleScreen';
import DataManagementScreen from '@/features/settings/DataManagementScreen';
import InventoryAdjustmentScreen from '@/features/inventory/InventoryAdjustmentScreen';
import EodReportScreen from '@/features/sales/EodReportScreen';
import AuditLogScreen from '@/features/audit/AuditLogScreen';
import OfflineQueueScreen from '@/features/offline/OfflineQueueScreen';
import { completeSetup, getSetupStatus } from '@/api/pos';
import { useFeatures } from '@/hooks/useFeatures';
import type { WizardState } from '@/features/setup/SetupWizard';
import type { AppRoute } from '@/components/AppLayout';
import '@/features/design/DesignSystem.css';
import '@/features/staff/StaffManagementScreen.css';
import '@/features/terminals/TerminalManagementScreen.css';
import '@/features/customers/CustomerManagementScreen.css';
import '@/features/sales/VoidOrdersScreen.css';
import '@/features/auth/StaffLoginScreen.css';
import '@/features/inventory/InventoryAdjustmentScreen.css';
import '@/features/audit/AuditLogScreen.css';
import '@/features/currency/ExchangeRateScreen.css';
import '@/features/offline/OfflineQueueScreen.css';
import '@/features/sales/EodReportScreen.css';

/**
 * Root app component.
 *
 * On mount, checks if the Setup Wizard has already been completed
 * (via the IPC bridge). If not, shows the wizard. When the wizard
 * completes, the chosen preset + features are persisted to SQLite.
 *
 * After setup, the app renders with a sidebar navigation that lets
 * the user switch between screens.
 */
export default function App() {
  return (
    <ThemeProvider>
      <AuthProvider>
        <ToastProvider>
          <AppShell />
        </ToastProvider>
      </AuthProvider>
    </ThemeProvider>
  );
}

function AppShell() {
  const [loading, setLoading] = useState(true);
  const [hasCompletedSetup, setHasCompletedSetup] = useState(false);
  const [currentRoute, setCurrentRoute] = useState<AppRoute>('products');
  const { enabled, loaded: featuresLoaded } = useFeatures();
  const { session } = useAuth();

  // On mount, check if setup was already completed.
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const status = await getSetupStatus();
        if (!cancelled) {
          setHasCompletedSetup(status.completed);
        }
      } catch {
        // IPC unavailable (e.g. running outside Tauri in dev).
        // Default to showing wizard.
        if (!cancelled) {
          setHasCompletedSetup(false);
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    })();
    return () => { cancelled = true; };
  }, []);

  const handleComplete = useCallback(async (state: WizardState) => {
    // Persist to SQLite via Tauri IPC bridge.
    try {
      await completeSetup({
        preset: state.preset ?? 'custom',
        features: Object.keys(state.features).filter(
          (k) => state.features[k],
        ),
      });
    } catch (err) {
      console.error('Failed to persist setup:', err);
    }
    setHasCompletedSetup(true);
  }, []);

  const handleSkip = useCallback(() => {
    setHasCompletedSetup(true);
  }, []);

  const handleNavigate = useCallback((route: AppRoute) => {
    setCurrentRoute(route);
  }, []);

  // Show a minimal loading state while checking setup status.
  if (loading) {
    return (
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          minHeight: '100dvh',
          color: 'var(--color-fg-secondary)',
          fontFamily: 'var(--font-sans)',
          fontSize: 'var(--text-base)',
        }}
      >
        Loading…
      </div>
    );
  }

  // Show the Setup Wizard until completed.
  if (!hasCompletedSetup) {
    return (
      <SetupWizard onComplete={handleComplete} onSkip={handleSkip} />
    );
  }

  // Require staff login before showing the main app.
  if (!session) {
    return (
      <StaffLoginScreen />
    );
  }

  // Main app with sidebar navigation.
  // Pass enabled features so the sidebar can hide feature-gated nav items.
  return (
    <AppLayout
      route={currentRoute}
      onNavigate={handleNavigate}
      {...(featuresLoaded ? { enabledFeatures: enabled } : {})}
    >
      {currentRoute === 'sales' && <PosScreen />}
      {currentRoute === 'sales-history' && <SalesHistoryScreen />}
      {currentRoute === 'sales-dashboard' && <SalesDashboardScreen />}
      {currentRoute === 'tax-config' && <TaxConfigurationScreen />}
      {currentRoute === 'products' && <ProductLookupScreen />}
      {currentRoute === 'categories' && <CategoryManagementScreen />}
      {currentRoute === 'data-management' && <DataManagementScreen />}
      {currentRoute === 'features' && <FeatureToggleScreen />}
      {currentRoute === 'inventory' && <ProductManagementScreen />}
      {currentRoute === 'inventory-adjustment' && <InventoryAdjustmentScreen />}
      {currentRoute === 'design' && <DesignSystem />}
      {currentRoute === 'orders' && <VoidOrdersScreen />}
      {currentRoute === 'customers' && <CustomerManagementScreen />}
      {currentRoute === 'staff' && <StaffManagementScreen />}
      {currentRoute === 'terminals' && <TerminalManagementScreen />}
      {currentRoute === 'eod-report' && <EodReportScreen />}
      {currentRoute === 'audit-log' && <AuditLogScreen />}
      {currentRoute === 'exchange-rates' && <ExchangeRateScreen />}
      {currentRoute === 'offline-queue' && <OfflineQueueScreen />}
      {currentRoute === 'settings' && <SettingsPage />}
    </AppLayout>
  );
}
