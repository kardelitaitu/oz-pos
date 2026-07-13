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
import { getLicenseStatus } from '@/api/license';
import LicenseActivationScreen from '@/features/auth/LicenseActivationScreen';
import CreatePinScreen from '@/features/auth/CreatePinScreen';

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
  const [hasActiveLicense, setHasActiveLicense] = useState(false);
  const [licenseError, setLicenseError] = useState<string | null>(null);
  const [currentRoute, setCurrentRoute] = useState<AppRoute>('products');
  const { enabled, loaded: featuresLoaded } = useFeatures();
  const { session, logout } = useAuth();
  const { activeWorkspace } = useWorkspace();
  const { goToWorkspacePicker } = useWorkspaceNav();
  const { isKdsKiosk } = useTerminalProfile();
  const { addToast } = useToast();
  // Stable ref so the mount effect below can call addToast without
  // listing it as a dependency (which would cause the effect to re-run
  // whenever the toast context re-creates its callback reference, resetting
  // hasActiveLicense back to false mid-flow).
  const addToastRef = useRef(addToast);
  addToastRef.current = addToast;

  useIdleTimer(() => {
    if (activeWorkspace) {
      goToWorkspacePicker();
      addToast({
        type: 'info',
        message: 'Returned to workspace picker due to inactivity. Configure auto-lock from Settings.',
      });
    } else if (session) {
      addToast({
        type: 'info',
        message: 'Automatic logout enabled. Configure from Settings.',
      });
      logout();
    }
  });

  // On mount, check license status and whether setup was already completed.
  // addToastRef (not addToast) is used so this effect runs exactly once and
  // cannot be re-triggered by a reference change in the toast context.
  //
  // Decision logic:
  //   • Fresh install (setup NOT done): the license gate applies. No active
  //     license → ActivationFlow (activate license + create owner account).
  //   • Existing install (setup DONE, user data present): always let the user
  //     through. License issues (expired, grace period, invalid) surface as a
  //     non-blocking warning toast — never as a forced re-activation screen.
  //     Forcing re-activation on an existing install would attempt to create a
  //     second owner account (which the backend rejects) and is confusing.
  //   • Dev mode (import.meta.env.DEV): skip the Rust license check entirely
  //     and always report active. Saves the rebuild-Rust step during UI work.
  useEffect(() => {
    // ── Dev-mode bypass ────────────────────────────────────────
    // In Vite dev mode, the Rust backend may not have been rebuilt
    // with the debug_assertions fix, causing a stale Missing/Expired
    // status and an annoying toast on every F5. Skip the IPC call
    // entirely and assume the license is valid.
    if (import.meta.env.DEV) {
      setHasCompletedSetup(true);
      setHasActiveLicense(true);
      setLoading(false);
      return;
    }

    let cancelled = false;
    (async () => {
      try {
        const [licenseStatus, status] = await Promise.all([
          getLicenseStatus(),
          getSetupStatus(),
        ]);

        if (!cancelled) {
          setHasCompletedSetup(status.completed);

          if (status.completed) {
            // ── Existing install ───────────────────────────────────────
            // Always let the user through to the login screen; surface
            // license issues as toasts so they can renew from Settings.
            setHasActiveLicense(true);
            if (licenseStatus.status === 'gracePeriod') {
              addToastRef.current({ type: 'warning', message: licenseStatus.message ?? 'License is in grace period.' });
            } else if (!licenseStatus.is_active) {
              addToastRef.current({ type: 'warning', message: licenseStatus.message ?? 'License is inactive. Please renew from Settings.' });
            }
          } else {
            // ── Fresh install ──────────────────────────────────────────
            // Respect the license gate; show ActivationFlow if not active.
            setHasActiveLicense(licenseStatus.is_active);
            if (licenseStatus.status === 'gracePeriod') {
              addToastRef.current({ type: 'warning', message: licenseStatus.message ?? 'License is in grace period.' });
            } else if (!licenseStatus.is_active && licenseStatus.status !== 'missing') {
              setLicenseError(licenseStatus.message);
            }
          }
        }
      } catch (err) {
        if (!cancelled) {
          // On any startup error, let the user through rather than blocking
          // them with the activation screen. Existing data should not be
          // gated behind a license check that failed for a transient reason.
          setHasActiveLicense(true);
          setHasCompletedSetup(true);
          console.error('License verification failed:', err);
          addToastRef.current({ type: 'error', message: 'Could not verify license status. Check your connection.' });
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    })();
    return () => { cancelled = true; };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []); // run once on mount — addToastRef keeps the callback current

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
      default_currency: state.default_currency,
    });
    setHasCompletedSetup(true);
  }, []);

  const handleSkip = useCallback(() => {
    dismissSetupWizard().catch(console.error);
    setHasCompletedSetup(true);
  }, []);

  /**
   * Called when the activation flow finishes (license activated + owner
   * account created). Marks setup as dismissed so the wizard is not
   * shown — users land directly on the workspace picker.
   */
  const handleActivationComplete = useCallback(() => {
    dismissSetupWizard().catch(console.error);
    setHasCompletedSetup(true);
    setHasActiveLicense(true);
  }, []);

  // ── F11 toggles fullscreen across all workpaces ───────────────
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

  if (!hasActiveLicense) {
    return (
      <ActivationFlow
        initialError={licenseError}
        onComplete={handleActivationComplete}
      />
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

/**
 * Manages the license-activation → owner-PIN-creation flow locally
 * so that the parent (AppShell) does not need to synchronise two
 * state variables across the transition boundary.
 */
function ActivationFlow({
  initialError,
  onComplete,
}: {
  initialError: string | null;
  onComplete: () => void;
}) {
  const [step, setStep] = useState<'activate' | 'bootstrap'>('activate');

  if (step === 'activate') {
    return (
      <LicenseActivationScreen
        initialError={initialError}
        onActivated={() => setStep('bootstrap')}
      />
    );
  }

  return <CreatePinScreen onCreated={onComplete} />;
}
