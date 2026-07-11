import { useState, useEffect, useCallback, useRef } from 'react';
import { Localized } from '@fluent/react';
import { useAuth } from '@/contexts/AuthContext';
import { useWorkspace } from '@/contexts/WorkspaceContext';
import TabletAppLayout from './TabletAppLayout';
import SetupWizard from '@/features/setup/SetupWizard';
import StaffLoginScreen from '@/features/auth/StaffLoginScreen';
import WorkspaceHome from '@/features/workspaces/WorkspaceHome';
import { completeSetup, dismissSetupWizard, getSetupStatus } from '@/api/settings';
import { useFeatures } from '@/hooks/useFeatures';
import { getPage, isPageAccessible } from '@/platform/ui/page-registry';
import PermissionDenied from '@/components/PermissionDenied';
import type { WizardState } from '@/features/setup/SetupWizard';
import RetailPosScreen from '@/features/retail/RetailPosScreen';
import PosScreen from '@/features/sales/PosScreen';
import KdsScreen from '@/features/kds/KdsScreen';

/**
 * Tablet-optimised application shell.
 *
 * ADR #4 Phase 3b: Uses WorkspaceContext for device-bound auto-boot
 * and dynamic tab bar from workspace_type_screens. Falls back to
 * workspace picker when no instance is selected.
 */
export default function TabletAppShell() {
  const [loading, setLoading] = useState(true);
  const [hasCompletedSetup, setHasCompletedSetup] = useState(false);
  const [currentRoute, setCurrentRoute] = useState('pos');
  const { enabled, loaded: featuresLoaded } = useFeatures();
  const { session } = useAuth();
  // ADR #4 Phase 3b: use WorkspaceContext for device-bound auto-boot.
  const {
    activeWorkspace,
    workspaceScreens,
  } = useWorkspace();

  // Navigate to workspace-appropriate route on selection.
  const prevWorkspaceRef = useRef(activeWorkspace);
  useEffect(() => {
    if (prevWorkspaceRef.current !== undefined && prevWorkspaceRef.current !== activeWorkspace) {
      const workspaceRoute: Record<string, string> = {
        'restaurant-pos': 'pos',
        'store-pos': 'pos',
        kds: 'kds',
        inventory: 'products',
        admin: 'settings',
      };
      setCurrentRoute(workspaceRoute[activeWorkspace ?? ''] ?? 'pos');
    }
    prevWorkspaceRef.current = activeWorkspace;
  }, [activeWorkspace]);

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

  const userRole = session?.role_name ?? '';

  const handleNavigate = useCallback((route: string) => {
    const target = getPage(route);
    if (target && !isPageAccessible(target, userRole)) {
      const accessiblePages = ['pos', 'products', 'sales-history', 'sales-dashboard'];
      const fallback = accessiblePages.find((r) => {
        const p = getPage(r);
        return p && isPageAccessible(p, userRole);
      }) ?? 'products';
      setCurrentRoute(fallback);
      return;
    }
    setCurrentRoute(route);
  }, [userRole]);

  const handleComplete = useCallback(async (state: WizardState) => {
    await completeSetup({
      preset: state.preset ?? 'custom',
      features: Object.keys(state.features).filter(
        (k) => state.features[k],
      ),
      default_currency: state.default_currency,
    });
    setHasCompletedSetup(true);
  }, []);

  const handleSkip = useCallback(() => {
    dismissSetupWizard().catch(console.error);
    setHasCompletedSetup(true);
  }, []);

  if (loading) {
    return (
      <div className="tablet-shell"
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
        <Localized id="shared-loading">Loading&hellip;</Localized>
      </div>
    );
  }

  if (!session) {
    return <StaffLoginScreen />;
  }

  if (!hasCompletedSetup) {
    return <SetupWizard onComplete={handleComplete} onSkip={handleSkip} onLaunch={() => setHasCompletedSetup(true)} />;
  }

  // ADR #4 Phase 3b: Workspace routing — same pattern as desktop AppShell.
  // If no workspace is active, show the picker. Fullscreen types render
  // directly. Sidebar types use TabletAppLayout with dynamic tabs.

  if (!activeWorkspace) {
    return (
      <div className="workspace-home-wrapper">
        <WorkspaceHome />
      </div>
    );
  }

  // Fullscreen workspaces — render without the tab bar.
  if (activeWorkspace === 'restaurant-pos') {
    return (
      <div className="workspace-fullscreen">
        <PosScreen onNavigate={handleNavigate} />
      </div>
    );
  }

  if (activeWorkspace === 'store-pos') {
    return (
      <div className="workspace-fullscreen">
        <RetailPosScreen onNavigate={handleNavigate} />
      </div>
    );
  }

  if (activeWorkspace === 'kds') {
    return (
      <div className="workspace-fullscreen">
        <KdsScreen />
      </div>
    );
  }

  // Sidebar-type workspaces (inventory, admin) — use TabletAppLayout
  // with a dynamic bottom tab bar from workspace_type_screens.
  const pageRegistration = getPage(currentRoute);
  const PageComponent = pageRegistration?.component ?? null;
  const pageDenied = pageRegistration && !isPageAccessible(pageRegistration, userRole);

  return (
    <TabletAppLayout
      route={currentRoute}
      onNavigate={handleNavigate}
      {...(featuresLoaded ? { enabledFeatures: enabled, userRole } : { userRole })}
      workspaceScreens={workspaceScreens}
    >
      {pageDenied ? (
        <PermissionDenied
          action={pageRegistration!.label}
          requiredRole={pageRegistration!.requiredRole!}
        />
      ) : PageComponent ? (
        <PageComponent />
      ) : null}
    </TabletAppLayout>
  );
}
