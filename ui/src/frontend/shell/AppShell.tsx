import { useState, useEffect, useCallback } from 'react';
import { useAuth } from '@/contexts/AuthContext';
import AppLayout, { type AppRoute } from './AppLayout';
import SetupWizard from '@/features/setup/SetupWizard';
import StaffLoginScreen from '@/features/auth/StaffLoginScreen';
import { completeSetup, getSetupStatus } from '@/api/settings';
import { useFeatures } from '@/hooks/useFeatures';
import { getPage, isPageAccessible } from '@/platform/ui/page-registry';
import PermissionDenied from '@/components/PermissionDenied';
import type { WizardState } from '@/features/setup/SetupWizard';

/**
 * Application shell — handles setup wizard flow, auth gates,
 * and renders the main AppLayout with registry-based page routing.
 */
export default function AppShell() {
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

  const userRole = session?.role_name;

  const handleNavigate = useCallback((route: AppRoute) => {
    const target = getPage(route);
    if (target && !isPageAccessible(target, userRole)) {
      const accessiblePages = ['sales', 'products', 'sales-history', 'sales-dashboard'];
      const fallback = accessiblePages.find((r) => {
        const p = getPage(r);
        return p && isPageAccessible(p, userRole);
      }) ?? 'products';
      setCurrentRoute(fallback);
      return;
    }
    setCurrentRoute(route);
  }, [userRole]);

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

  if (!hasCompletedSetup) {
    return (
      <SetupWizard onComplete={handleComplete} onSkip={handleSkip} />
    );
  }

  if (!session) {
    return (
      <StaffLoginScreen />
    );
  }

  // Render the current page from the registry, or null if not found.
  const pageRegistration = getPage(currentRoute);
  const PageComponent = pageRegistration?.component ?? null;
  const pageDenied = pageRegistration && !isPageAccessible(pageRegistration, userRole);

  return (
    <AppLayout
      route={currentRoute}
      onNavigate={handleNavigate}
      {...(featuresLoaded ? { enabledFeatures: enabled, userRole } : { userRole })}
    >
      {pageDenied ? (
        <PermissionDenied
          action={pageRegistration!.label}
          requiredRole={pageRegistration!.requiredRole!}
        />
      ) : PageComponent ? (
        <PageComponent />
      ) : null}
    </AppLayout>
  );
}
