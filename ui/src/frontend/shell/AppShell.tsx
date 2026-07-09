import { useState, useEffect, useCallback, useRef } from 'react';
import { Localized } from '@fluent/react';
import { useAuth } from '@/contexts/AuthContext';
import { useToast } from '@/frontend/shared/Toast';
import { useWorkspace } from '@/contexts/WorkspaceContext';
import { useIdleTimer } from '@/hooks/useIdleTimer';
import { useWorkspaceNav } from '@/hooks/useWorkspaceNav';
import { useFullscreen } from '@/hooks/useFullscreen';
import AppLayout, { type AppRoute } from './AppLayout';
import SetupWizard from '@/features/setup/SetupWizard';
import StaffLoginScreen from '@/features/auth/StaffLoginScreen';
import WorkspaceHome from '@/features/workspaces/WorkspaceHome';
import { completeSetup, dismissSetupWizard, getSetupStatus } from '@/api/settings';
import { useFeatures } from '@/hooks/useFeatures';
import { useTerminalProfile } from '@/hooks/useTerminalProfile';
import { getPage, isPageAccessible } from '@/platform/ui/page-registry';
import PermissionDenied from '@/components/PermissionDenied';
import type { WizardState } from '@/features/setup/SetupWizard';
import RetailPosScreen from '@/features/retail/RetailPosScreen';
import PosScreen from '@/features/sales/PosScreen';
import KdsScreen from '@/features/kds/KdsScreen';

// ── Workspace navigation keyboard shortcuts ───────────────────────
// Escape: return to workspace picker (only when no modal is open).
// Ctrl+Shift+Escape: global shortcut to return to workspace picker
// regardless of modals.
function useWorkspaceNavShortcuts(active: string | null, onBack: () => void) {
  useEffect(() => {
    if (!active) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        // Ctrl+Shift+Escape always returns to the picker, bypassing modals.
        if (e.ctrlKey && e.shiftKey) {
          onBack();
        } else if (!document.querySelector('.modal-overlay')) {
          onBack();
        }
      }
    };
    document.addEventListener('keydown', handler);
    return () => document.removeEventListener('keydown', handler);
  }, [active, onBack]);
}

/**
 * Application shell — handles setup wizard flow, auth gates,
 * and renders the main AppLayout with registry-based page routing.
 */
export default function AppShell() {
  const [loading, setLoading] = useState(true);
  const [hasCompletedSetup, setHasCompletedSetup] = useState(false);
  const [currentRoute, setCurrentRoute] = useState<AppRoute>('products');
  const { enabled, loaded: featuresLoaded } = useFeatures();
  const { session, logout } = useAuth();
  const { activeWorkspace } = useWorkspace();
  const { goToWorkspacePicker } = useWorkspaceNav();
  const { isKdsKiosk } = useTerminalProfile();

  useIdleTimer(() => {
    if (activeWorkspace) {
      goToWorkspacePicker();
      addToast({
        type: 'info',
        message: 'Returned to workspace picker due to inactivity',
      });
    } else if (session) {
      logout();
    }
  });

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

  // Navigate to workspace-appropriate route on selection.
  const prevWorkspaceRef = useRef(activeWorkspace);
  useEffect(() => {
    if (prevWorkspaceRef.current !== undefined && prevWorkspaceRef.current !== activeWorkspace) {
      const workspaceRoute: Record<string, string> = {
        'restaurant-pos': 'sales',
        'store-pos': 'products',
        kds: 'kds',
        inventory: 'inventory',
        admin: 'settings',
      };
      setCurrentRoute(workspaceRoute[activeWorkspace ?? ''] ?? 'products');
    }
    prevWorkspaceRef.current = activeWorkspace;
  }, [activeWorkspace]);

  const handleComplete = useCallback(async (state: WizardState) => {
    await completeSetup({
      preset: state.preset ?? 'custom',
      features: Object.keys(state.features).filter(
        (k) => state.features[k],
      ),
    });
    setHasCompletedSetup(true);
  }, []);

  const handleSkip = useCallback(() => {
    dismissSetupWizard().catch(console.error);
    setHasCompletedSetup(true);
  }, []);

  // ── F11 toggles fullscreen across all workpaces ───────────────
  const { addToast } = useToast();
  useFullscreen((isFullscreen) => {
    addToast({
      type: 'info',
      message: isFullscreen
        ? 'Fullscreen mode enabled'
        : 'Fullscreen mode disabled',
    });
  });

  // ── Escape key navigates back to workspace picker ────────────

  const handleBackToPicker = useCallback(() => {
    goToWorkspacePicker();
  }, [goToWorkspacePicker]);

  useWorkspaceNavShortcuts(activeWorkspace, handleBackToPicker);

  const userRole = session?.role_name ?? '';

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
        <Localized id="shared-loading">Loading&hellip;</Localized>
      </div>
    );
  }

  if (!session) {
    return (
      <StaffLoginScreen />
    );
  }

  if (!hasCompletedSetup) {
    return (
      <SetupWizard onComplete={handleComplete} onSkip={handleSkip} onLaunch={() => setHasCompletedSetup(true)} />
    );
  }

  // ── KDS Kiosk — force KDS route, hide header, no workspace picker ──
  if (isKdsKiosk) {
    return (
      <div className="workspace-fullscreen">
        <div className="kds-workspace">
          <KdsScreen />
        </div>
      </div>
    );
  }

  if (!activeWorkspace) {
    return (
      <div className="workspace-home-wrapper">
        <WorkspaceHome />
      </div>
    );
  }

  // Render the current page from the registry, or null if not found.
  const pageRegistration = getPage(currentRoute);
  const PageComponent = pageRegistration?.component ?? null;
  const pageDenied = pageRegistration && !isPageAccessible(pageRegistration, userRole);

  // Workspace fullscreen — restaurant POS hides the sidebar.
  // KDS is a separate workspace screen, navigated to via the chef button in PosScreen.
  if (activeWorkspace === 'restaurant-pos') {
    if (currentRoute === 'kds') {
      return (
        <div className="workspace-fullscreen">
          <div className="kds-workspace">
            <div className="kds-workspace-header">
              <button
                className="kds-workspace-back"
                onClick={() => handleNavigate('sales')}
              >
                <Localized id="back">
                  <span>&larr; Back</span>
                </Localized>
              </button>
            </div>
            <KdsScreen />
          </div>
        </div>
      );
    }
    return (
      <div className="workspace-fullscreen">
        <PosScreen onNavigate={handleNavigate} />
      </div>
    );
  }

  // Workspace fullscreen — retail POS with its own layout.
  // KDS is a separate workspace screen, navigated to via F12 or function bar.
  if (activeWorkspace === 'store-pos') {
    if (currentRoute === 'kds') {
      return (
        <div className="workspace-fullscreen">
          <div className="kds-workspace">
            <div className="kds-workspace-header">
              <button
                className="kds-workspace-back"
                onClick={() => handleNavigate('products')}
              >
                <Localized id="back">
                  <span>&larr; Back</span>
                </Localized>
              </button>
            </div>
            <KdsScreen />
          </div>
        </div>
      );
    }
    return (
      <div className="workspace-fullscreen">
        <RetailPosScreen onNavigate={handleNavigate} />
      </div>
    );
  }

  // Fullscreen workspace — KDS.
  if (activeWorkspace === 'kds') {
    return (
      <div className="workspace-fullscreen">
        <KdsScreen />
      </div>
    );
  }

  // Fullscreen pages (e.g. Kiosk mode) render without AppLayout wrapper.
  if (pageRegistration?.fullscreen) {
    return pageDenied ? (
      <PermissionDenied
        action={pageRegistration!.label}
        requiredRole={pageRegistration!.requiredRole!}
      />
    ) : PageComponent ? (
      <PageComponent />
    ) : null;
  }

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
