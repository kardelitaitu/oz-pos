import { useState, useEffect, useCallback } from 'react';
import { ThemeProvider } from '@/components/ThemeProvider';
import AppLayout from '@/components/AppLayout';
import SetupWizard from '@/features/setup/SetupWizard';
import DesignSystem from '@/features/design/DesignSystem';
import PosScreen from '@/features/sales/PosScreen';
import ProductLookupScreen from '@/features/products/ProductLookupScreen';
import { completeSetup, getSetupStatus } from '@/api/pos';
import type { WizardState } from '@/features/setup/SetupWizard';
import type { AppRoute } from '@/components/AppLayout';
import '@/features/design/DesignSystem.css';

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
      <AppShell />
    </ThemeProvider>
  );
}

function AppShell() {
  const [loading, setLoading] = useState(true);
  const [hasCompletedSetup, setHasCompletedSetup] = useState(false);
  const [currentRoute, setCurrentRoute] = useState<AppRoute>('products');

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

  // Main app with sidebar navigation.
  return (
    <AppLayout route={currentRoute} onNavigate={handleNavigate}>
      {currentRoute === 'sales' && <PosScreen />}
      {currentRoute === 'products' && <ProductLookupScreen />}
      {currentRoute === 'design' && <DesignSystem />}
    </AppLayout>
  );
}
