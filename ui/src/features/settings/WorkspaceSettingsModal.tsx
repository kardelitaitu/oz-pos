import { useEffect, useRef, useState, type ReactNode } from 'react';
import { createPortal } from 'react-dom';
import { Localized } from '@fluent/react';
import { useExitAnimation } from '@/hooks/useExitAnimation';
import { useFocusTrap } from '@/hooks/useFocusTrap';
import { useAuth } from '@/contexts/AuthContext';
import { useWorkspace } from '@/contexts/WorkspaceContext';
import { Button } from '@/components/Button';
import ErrorBoundary from '@/components/ErrorBoundary';
import {
  WorkspaceStorePosSettings,
  WorkspaceRestaurantPosSettings,
  WorkspaceKdsSettings,
  WorkspaceInventorySettings,
  TerminalPreferencesCard,
} from '@/features/settings/workspace-cards';
import {
  getNestedDepth,
  onNestedDepthChange,
} from './nestedModalDepth';
import styles from './WorkspaceSettingsModal.module.css';

// ── Types ──────────────────────────────────────────────────────────

export type WorkspaceType = 'store-pos' | 'restaurant-pos' | 'kds' | 'inventory';
export type ModalPresentation = 'overlay' | 'slideover';

export interface WorkspaceSettingsModalProps {
  /** Whether the modal is open. */
  open: boolean;
  /** Called to close the modal (after exit animation completes). */
  onClose: () => void;
  /** Which workspace card to render. */
  workspaceType: WorkspaceType;
  /** Active terminal ID for hardware-scoped settings. */
  terminalId?: string;
  /** Visual presentation variant. */
  presentation?: ModalPresentation;
}

// ── Workspace card adapter ─────────────────────────────────────────

function renderWorkspaceCard(
  workspaceType: WorkspaceType,
  terminalId: string | undefined,
): ReactNode {
  const cardProps = { variant: 'modal' as const, terminalId: terminalId ?? '' };

  switch (workspaceType) {
    case 'restaurant-pos':
      return <WorkspaceRestaurantPosSettings {...cardProps} />;
    case 'kds':
      return <WorkspaceKdsSettings {...cardProps} />;
    case 'inventory':
      return <WorkspaceInventorySettings {...cardProps} />;
    default:
      return <WorkspaceStorePosSettings {...cardProps} />;
  }
}

function useWorkspaceOptional() {
  try {
    return useWorkspace();
  } catch {
    return null;
  }
}

// ── Component ──────────────────────────────────────────────────────

/**
 * Tier 2 contextual settings modal (ADR #22 Phase 4).
 *
 * Opens via F10 inside a workspace. Renders the appropriate shared
 * workspace card scoped to the active workspace. Handles role-based
 * content gating (Cashiers see only TerminalPreferencesCard),
 * isolates POS hotkeys while open via aria-modal="true", traps focus,
 * animates entry/exit, and provides an "Admin Settings" shortcut
 * to the Tier 1 hub.
 */
export default function WorkspaceSettingsModal({
  open,
  onClose,
  workspaceType,
  terminalId,
  presentation = 'overlay',
}: WorkspaceSettingsModalProps) {
  const { isManager } = useAuth();
  const workspaceCtx = useWorkspaceOptional();
  const panelRef = useRef<HTMLDivElement>(null);

  // ── Nested modal depth (React state for reactivity) ─────────
  const [nestedDepth, setNestedDepth] = useState(() => getNestedDepth());

  useEffect(() => {
    return onNestedDepthChange((depth) => setNestedDepth(depth));
  }, []);

  // ── Exit animation ──────────────────────────────────────────
  const exit = useExitAnimation(open, onClose, 200);

  // ── Focus trap (suspended when nested modal is open) ────────
  const trapActive = exit.shouldRender && !exit.exiting && nestedDepth === 0;
  useFocusTrap(panelRef, trapActive, () => exit.requestClose());

  // ── Reset nested depth on unmount ───────────────────────────
  useEffect(() => {
    return () => {
      // Reset depth when this modal unmounts
      setNestedDepth(0);
    };
  }, []);

  // ── Admin Settings shortcut ─────────────────────────────────
  const handleAdminSettings = () => {
    // Switch active workspace to admin and navigate to Tier 1 settings hub
    if (workspaceCtx?.setActiveWorkspace) {
      workspaceCtx.setActiveWorkspace('admin');
    }
    window.location.hash = '#/settings';
    onClose();
  };

  // ── Don't render if shouldRender is false ───────────────────
  if (!exit.shouldRender) return null;

  // ── Role-gated card selection ───────────────────────────────
  const showFullCard = isManager;

  return createPortal(
    <div
      className={`${styles['backdrop']} ${exit.exiting ? styles['backdrop--exiting'] : ''}`}
      onClick={(e) => {
        if (e.target === e.currentTarget && nestedDepth === 0) {
          exit.requestClose();
        }
      }}
      role="presentation"
      aria-hidden={!open}
    >
      <div
        ref={panelRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby="workspace-settings-title"
        className={`${styles['panel']} ${styles[`panel--${presentation}`]} ${exit.exiting ? styles['panel--exiting'] : ''}`}
      >
        {/* ── Header ─────────────────────────────────── */}
        <div className={styles['header']}>
          <h2 id="workspace-settings-title" className={styles['title']}>
            <Localized id="workspace-modal-title">Workspace Settings</Localized>
          </h2>

          <div className={styles['header-actions']}>
            {showFullCard && (
              <Button
                variant="secondary"
                onClick={handleAdminSettings}
              >
                <Localized id="workspace-modal-admin-settings">
                  Admin Settings ↗
                </Localized>
              </Button>
            )}

            <button
              type="button"
              className={styles['close-btn']}
              onClick={() => exit.requestClose()}
              aria-label="Close settings"
            >
              <svg
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
                aria-hidden="true"
                width="20"
                height="20"
              >
                <line x1="18" y1="6" x2="6" y2="18" />
                <line x1="6" y1="6" x2="18" y2="18" />
              </svg>
            </button>
          </div>
        </div>

        {/* ── Body ──────────────────────────────────── */}
        <div className={styles['body']}>
          <ErrorBoundary>
            {showFullCard ? (
              renderWorkspaceCard(workspaceType, terminalId)
            ) : (
              <TerminalPreferencesCard
                variant="modal"
                terminalId={terminalId ?? ''}
              />
            )}
          </ErrorBoundary>
        </div>

        {/* ── Footer role indicator ─────────────────── */}
        <div className={styles['footer']}>
          <span className={styles['role-badge']}>
            <Localized
              id={
                showFullCard
                  ? 'workspace-modal-role-manager'
                  : 'workspace-modal-role-cashier'
              }
            >
              {showFullCard ? 'Manager' : 'Cashier'}
            </Localized>
          </span>
        </div>
      </div>
    </div>,
    document.body,
  );
}
