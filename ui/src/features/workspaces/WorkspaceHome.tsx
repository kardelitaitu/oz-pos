import { useCallback, useMemo, useRef, useEffect, useState } from 'react';
import { useWorkspace } from '@/contexts/WorkspaceContext';
import { useAuth } from '@/contexts/AuthContext';
import { useFullscreen } from '@/hooks/useFullscreen';
import { Localized, useLocalization } from '@fluent/react';
import { Modal } from '@/components/Modal';
import { WorkspaceIcon } from '@/components/WorkspaceIcon';
import { RoleIcon } from '@/components/RoleIcon';
import type { LoginSessionDto } from '@/api/staff';
import './WorkspaceHome.css';

// ── Per-workspace accent color classes ────────────────────────────

const WS_COLORS: Record<string, string> = {
  'restaurant-pos': 'ws-color-restaurant-pos',
  'store-pos': 'ws-color-store-pos',
  kds: 'ws-color-kds',
  inventory: 'ws-color-inventory',
  admin: 'ws-color-admin',
};

// ── Workspace sort order ──────────────────────────────────────────

const WS_ORDER: Record<string, number> = {
  'restaurant-pos': 1,
  'store-pos': 2,
  kds: 3,
  inventory: 4,
  admin: 5,
};

// ── Icons ─────────────────────────────────────────────────────────

function getIcon(key: string) {
  const known = ['restaurant-pos', 'store-pos', 'kds', 'inventory', 'admin', 'Loyalty', 'Marketing', 'Online Orders', 'Analytics'];
  if (!known.includes(key)) {
    console.warn(`WorkspaceHome: unknown workspace key "${key}" — using default icon`);
  }
  return <WorkspaceIcon wsKey={key} />;
}

// ── Skeleton ──────────────────────────────────────────────────────

function SkeletonGrid() {
  return (
    <div className="workspace-skeleton-grid" aria-label="Loading workspaces">
      {[1, 2, 3].map((i) => (
        <div key={i} className="workspace-skeleton-card">
          <div className="workspace-skeleton-icon" />
          <div className="workspace-skeleton-title" />
          <div className="workspace-skeleton-desc" />
          <div className="workspace-skeleton-desc" />
        </div>
      ))}
    </div>
  );
}

// ── Randomized multilingual greeting ────────────────────────────

const GREETINGS: { word: string; lang: string }[] = [
  { word: 'Hello', lang: 'English' },
  { word: 'Hola', lang: 'Spanish' },
  { word: 'Bonjour', lang: 'French' },
  { word: 'Ciao', lang: 'Italian' },
  { word: 'Konnichiwa', lang: 'Japanese' },
  { word: 'Annyeong', lang: 'Korean' },
  { word: 'Ni hao', lang: 'Chinese' },
  { word: 'Salaam', lang: 'Arabic' },
  { word: 'Sawasdee', lang: 'Thai' },
  { word: 'Zdravstvuyte', lang: 'Russian' },
  { word: 'Guten Tag', lang: 'German' },
  { word: 'Olá', lang: 'Portuguese' },
  { word: 'Namaste', lang: 'Hindi' },
  { word: 'Merhaba', lang: 'Turkish' },
  { word: 'Hej', lang: 'Swedish' },
  { word: 'Salut', lang: 'French' },
  { word: 'Hallo', lang: 'Dutch' },
  { word: 'Ahoj', lang: 'Czech' },
  { word: 'Selamat datang', lang: 'Indonesian' },
  { word: 'Sawubona', lang: 'Zulu' },
  { word: 'Shalom', lang: 'Hebrew' },
  { word: 'Jambo', lang: 'Swahili' },
];

function pickGreeting(): { word: string; lang: string } {
  return GREETINGS[Math.floor(Math.random() * GREETINGS.length)]!;
}

// ── Dummy coming-soon cards (placeholder for future workspaces) ──

const COMING_SOON_CARDS = [
  { name: 'Loyalty', description: 'Coming soon' },
  { name: 'Marketing', description: 'Coming soon' },
  { name: 'Online Orders', description: 'Coming soon' },
  { name: 'Analytics', description: 'Coming soon' },
];

// ── LogoutModal ───────────────────────────────────────────────────

function LogoutModal({
  open,
  onCancel,
  onConfirm,
}: {
  open: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  const { l10n } = useLocalization();

  return (
    <Modal
      open={open}
      onClose={onCancel}
      title={l10n.getString('workspace-home-logout-confirm-title')}
      footer={
        <div className="logout-confirm-actions">
          <button
            type="button"
            className="logout-confirm-cancel"
            onClick={onCancel}
          >
            <Localized id="workspace-home-logout-confirm-cancel">
              <span>Cancel</span>
            </Localized>
          </button>
          <button
            type="button"
            className="logout-confirm-confirm"
            onClick={onConfirm}
          >
            <Localized id="workspace-home-logout-confirm-confirm">
              <span>Logout</span>
            </Localized>
          </button>
        </div>
      }
    >
      <p className="logout-confirm-desc">
        <Localized id="workspace-home-logout-confirm-desc">
          <span>You will be returned to the login screen. Any unsaved work will be lost.</span>
        </Localized>
      </p>
    </Modal>
  );
}



// ── Role color map ────────────────────────────────────────────────

function getRoleColor(role: string): string {
  switch (role.toLowerCase()) {
    case 'owner':
    case 'role-owner':
    case 'admin':
    case 'role-admin':   return 'role-badge--owner';
    case 'manager':
    case 'role-manager': return 'role-badge--manager';
    case 'cashier':
    case 'role-cashier': return 'role-badge--cashier';
    case 'kitchen':
    case 'role-kitchen': return 'role-badge--kitchen';
    default:             return 'role-badge--default';
  }
}

// ── Layer 1: Background ──────────────────────────────────────────

function LayerBackground() {
  return (
    <div className="ws-layer-bg" aria-hidden="true">
      <div className="ws-layer-bg-gradient" />
      <div className="ws-layer-bg-particles">
        <div className="ws-particle" />
        <div className="ws-particle" />
        <div className="ws-particle" />
        <div className="ws-particle" />
        <div className="ws-particle" />
        <div className="ws-particle" />
      </div>
    </div>
  );
}

// ── Toolbar buttons (fullscreen, user profile, logout) ────────

function LayerFloatingButtons({
  session,
  displayName,
  roleName,
  l10n,
  toggleFullscreen,
  handleLogoutClick,
  error,
  retry,
  greeting,
}: {
  session: LoginSessionDto | null;
  displayName: string;
  roleName: string;
  l10n: ReturnType<typeof useLocalization>['l10n'];
  toggleFullscreen: () => void;
  handleLogoutClick: () => void;
  error: string | null;
  retry: () => void;
  greeting: { word: string; lang: string };
}) {
  return (
    <>
    {session && displayName && (
      <span className="ws-header-greeting" title={greeting.lang}>
        {greeting.word}, {displayName}
      </span>
    )}
    <div className="ws-header-buttons">
        <button
          type="button"
          className="workspace-home-fullscreen-btn"
          onClick={toggleFullscreen}
          aria-label={l10n.getString('workspace-home-fullscreen-aria')}
          title="F11"
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" width="18" height="18">
            <path d="M8 3H5a2 2 0 0 0-2 2v3m18 0V5a2 2 0 0 0-2-2h-3m0 18h3a2 2 0 0 0 2-2v-3M3 16v3a2 2 0 0 0 2 2h3" />
          </svg>
        </button>
        {session && (
          <>
            <button type="button" className="workspace-home-user-profile" aria-label={l10n.getString('workspace-home-user-aria', { name: displayName })}>
              <div className="workspace-home-user-avatar">
                <div className="workspace-home-user-avatar-inner">
                  <RoleIcon role={roleName} size={16} />
                </div>
              </div>
              <div className="workspace-home-user-info">
                <span className="workspace-home-user-name">{displayName}</span>
                <span className={`workspace-home-user-role ${getRoleColor(roleName)}`}>{roleName}</span>
              </div>
            </button>
            <button type="button" className="workspace-home-logout-btn" onClick={handleLogoutClick}>
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" width="20" height="20">
                <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" />
                <polyline points="16 17 21 12 16 7" />
                <line x1="21" y1="12" x2="9" y2="12" />
              </svg>
              <Localized id="workspace-home-logout"><span>Logout</span></Localized>
            </button>
          </>
        )}
        {error && (
          <button
            type="button"
            className="workspace-home-logout-btn"
            onClick={retry}
            title="Retry"
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="16" height="16" aria-hidden="true">
              <polyline points="1 4 1 10 7 10" />
              <path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10" />
            </svg>
            <Localized id="workspace-home-retry-btn">
              <span>Retry</span>
            </Localized>
          </button>
        )}
    </div>
    </>
  );
}

// ── Component ─────────────────────────────────────────────────────

/** Workspace home screen — 5-layer architecture with role-based workspace selection grid. */
export default function WorkspaceHome() {
  const { l10n } = useLocalization();
  const { availableWorkspaces, loading, error, retry, setActiveWorkspace, lastWorkspace } = useWorkspace();
  const { session, logout } = useAuth();
  const gridRef = useRef<HTMLDivElement>(null);
  const ripplesRef = useRef<HTMLSpanElement[]>([]);
  const [showLogoutModal, setShowLogoutModal] = useState(false);
  const [exitingWorkspace, setExitingWorkspace] = useState<string | null>(null);

  const roleName = (session?.role_name ?? '').toLowerCase();

  // Sort workspaces for consistent order
  const sortedWorkspaces = useMemo(
    () =>
      [...availableWorkspaces].sort(
        (a, b) => (WS_ORDER[a.type_key] ?? 99) - (WS_ORDER[b.type_key] ?? 99),
      ),
    [availableWorkspaces],
  );

  const cashierOnly = useMemo(() => new Set(['restaurant-pos', 'store-pos']), []);
  const kitchenOnly = useMemo(() => new Set(['kds']), []);

  const canAccess = useCallback(
    (key: string) =>
      roleName === 'owner' ||
      roleName === 'role-owner' ||
      roleName === 'admin' ||
      roleName === 'role-admin' ||
      roleName === 'manager' ||
      roleName === 'role-manager' ||
      cashierOnly.has(key) ||
      (roleName === 'kitchen' && kitchenOnly.has(key)),
    [roleName, cashierOnly, kitchenOnly],
  );

  const greeting = useMemo(() => pickGreeting(), []);

  const displayName = session?.display_name ?? session?.role_name ?? '';

  // ── Loading state (no entrance animations) ────────────────────

  const showSkeleton = loading;

  // ── Fullscreen toggle ─────────────────────────────────────────
  const { toggleFullscreen } = useFullscreen();

  // ── Logout confirmation ────────────────────────────────────────

  const handleLogoutClick = useCallback(() => {
    setShowLogoutModal(true);
  }, []);

  const handleLogoutCancel = useCallback(() => {
    setShowLogoutModal(false);
  }, []);

  const handleLogoutConfirm = useCallback(() => {
    setShowLogoutModal(false);
    logout();
  }, [logout]);

  // ── Ripple cleanup on unmount ──────────────────────────────

  useEffect(() => {
    return () => {
      ripplesRef.current.forEach(r => r.remove());
      ripplesRef.current = [];
    };
  }, []);

  // ── Click ripple + immediate navigation ──────────────────────────
  //
  // Navigates on the same event loop tick — no 300ms exit delay.
  // The exiting state is set briefly for the CSS animation but the
  // component unmounts before it completes; on re-entry the state is
  // fresh from the useState initialiser.

  const handleCardClick = useCallback(
    (key: string, e: React.MouseEvent<HTMLButtonElement>) => {
      if (!canAccess(key)) return;
      if (error) return;
      if (exitingWorkspace) return;
      const card = e.currentTarget;
      const rect = card.getBoundingClientRect();

      // Ripple effect
      const ripple = document.createElement('span');
      ripple.className = 'workspace-card-ripple';
      const size = Math.max(rect.width, rect.height);
      const clickX = e.clientX !== 0 ? e.clientX : rect.left + rect.width / 2;
      const clickY = e.clientY !== 0 ? e.clientY : rect.top + rect.height / 2;
      const x = clickX - rect.left - size / 2;
      const y = clickY - rect.top - size / 2;
      ripple.style.width = ripple.style.height = `${size}px`;
      ripple.style.left = `${x}px`;
      ripple.style.top = `${y}px`;
      card.appendChild(ripple);
      ripplesRef.current.push(ripple);
      ripple.addEventListener('animationend', () => {
        ripple.remove();
        ripplesRef.current = ripplesRef.current.filter(r => r !== ripple);
      });
      setTimeout(() => {
        if (ripple.parentNode) {
          ripple.remove();
          ripplesRef.current = ripplesRef.current.filter(r => r !== ripple);
        }
      }, 600);

      // Navigate immediately — no exit animation delay
      setExitingWorkspace(key);
      setActiveWorkspace(key);
    },
    [canAccess, setActiveWorkspace, error, exitingWorkspace],
  );

  // ── Keyboard navigation ──────────────────────────────────────

  useEffect(() => {
    const grid = gridRef.current;
    if (!grid) return;

    const cards = grid.querySelectorAll<HTMLButtonElement>('.workspace-card:not(.workspace-card--disabled)');
    if (cards.length === 0) return;

    function focusCard(index: number) {
      const target = cards[index];
      if (target && !target.disabled) {
        target.focus();
      }
    }

    function getColumns(): number {
      const style = getComputedStyle(grid!);
      const gridCols = style.gridTemplateColumns.split(' ');
      return gridCols.length;
    }

    const handleKeyDown = (e: KeyboardEvent) => {
      // ── Quick-launch: number keys 1-9 select workspace by index ───
      if (e.key >= '1' && e.key <= '9' && !e.ctrlKey && !e.altKey && !e.metaKey) {
        const activeTag = document.activeElement?.tagName;
        if (activeTag === 'INPUT' || activeTag === 'TEXTAREA' || activeTag === 'SELECT') return;
        const idx = parseInt(e.key, 10) - 1;
        if (idx < cards.length) {
          e.preventDefault();
          const target = cards[idx];
          if (target && !target.disabled && !target.classList.contains('workspace-card--disabled')) {
            // Programmatic click won't create a ripple,
            // but we still need to activate the workspace.
            // dispatchEvent is used to trigger the React onClick handler.
            target.dispatchEvent(new MouseEvent('click', { bubbles: true }));
          }
        }
        return;
      }

      const active = document.activeElement;
      if (!active || !grid.contains(active)) return;

      let currentIndex = -1;
      for (let i = 0; i < cards.length; i++) {
        if (cards[i] === active) {
          currentIndex = i;
          break;
        }
      }
      if (currentIndex < 0) return;

      const cols = getColumns();

      switch (e.key) {
        case 'ArrowRight':
          e.preventDefault();
          if (currentIndex < cards.length - 1) focusCard(currentIndex + 1);
          break;
        case 'ArrowLeft':
          e.preventDefault();
          if (currentIndex > 0) focusCard(currentIndex - 1);
          break;
        case 'ArrowDown':
          e.preventDefault();
          if (currentIndex + cols < cards.length) focusCard(currentIndex + cols);
          break;
        case 'ArrowUp':
          e.preventDefault();
          if (currentIndex - cols >= 0) focusCard(currentIndex - cols);
          break;
        case 'Home':
          e.preventDefault();
          focusCard(0);
          break;
        case 'End':
          e.preventDefault();
          focusCard(cards.length - 1);
          break;
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [sortedWorkspaces, setActiveWorkspace, canAccess, error]);

  // ── RAF-throttled mousemove: glow & 3D tilt ───────────────────
  //
  // getBoundingClientRect forces a synchronous style recalculation, so we
  // throttle it to once per animation frame to prevent layout thrashing.

  const rafRef = useRef<number>(0);
  const lastMoveRef = useRef<{ card: HTMLButtonElement; clientX: number; clientY: number } | null>(null);

  const handleMouseMove = useCallback((e: React.MouseEvent<HTMLButtonElement>) => {
    const card = e.currentTarget;
    if (card.classList.contains('workspace-card--disabled') || card.classList.contains('workspace-card--exiting')) return;

    // Store the latest event data
    lastMoveRef.current = { card, clientX: e.clientX, clientY: e.clientY };

    // If no RAF is queued, schedule one
    if (!rafRef.current) {
      rafRef.current = requestAnimationFrame(() => {
        rafRef.current = 0;
        const data = lastMoveRef.current;
        if (!data) return;
        lastMoveRef.current = null;

        const { card: c, clientX, clientY } = data;
        const rect = c.getBoundingClientRect();

        // Glow position
        const x = ((clientX - rect.left) / rect.width) * 100;
        const y = ((clientY - rect.top) / rect.height) * 100;
        c.style.setProperty('--mouse-x', `${x}%`);
        c.style.setProperty('--mouse-y', `${y}%`);

        // 3D tilt: max ±6 degrees
        const rotateY = ((clientX - rect.left) / rect.width - 0.5) * 12;
        const rotateX = ((clientY - rect.top) / rect.height - 0.5) * -12;
        c.style.setProperty('--rotate-x', `${rotateX}deg`);
        c.style.setProperty('--rotate-y', `${rotateY}deg`);
      });
    }
  }, []);

  const handleMouseLeave = useCallback((e: React.MouseEvent<HTMLButtonElement>) => {
    const card = e.currentTarget;
    // Cancel any pending RAF to avoid stale updates
    if (rafRef.current) {
      cancelAnimationFrame(rafRef.current);
      rafRef.current = 0;
    }
    lastMoveRef.current = null;
    card.style.setProperty('--rotate-x', '0deg');
    card.style.setProperty('--rotate-y', '0deg');
  }, []);

  // Cleanup RAF on unmount
  useEffect(() => {
    return () => {
      if (rafRef.current) cancelAnimationFrame(rafRef.current);
    };
  }, []);

  // ── Clear stale focus on mount ─────────────────────────────────
  // When returning from a workspace (Escape), the active element from
  // the previous view may still be focused in the DOM, or the browser
  // may restore focus to a card. A stuck :focus-visible outline draws
  // over the hover glow and makes the card look unresponsive.
  useEffect(() => {
    if (document.activeElement && document.activeElement instanceof HTMLElement) {
      document.activeElement.blur();
    }
  }, []);

  // ── Shared floating layer props ────────────────────────────────

  const floatingProps = {
    session,
    displayName,
    roleName,
    l10n,
    toggleFullscreen,
    handleLogoutClick,
    error,
    retry,
    greeting,
  };

  // ── Loading state ────────────────────────────────────────────

  if (showSkeleton) {
    return (
      <div className="workspace-home">
        {/* Layer 1: Background */}
        <LayerBackground />

        {/* Layer 2+3: Content container + content */}
        <div className="ws-layer-content">
          <div className="ws-header">
            <LayerFloatingButtons {...floatingProps} />
          </div>
          <div className="ws-main">
            <header className="workspace-home-header" />
            <SkeletonGrid />
          </div>
          <div className="ws-footer" />
        </div>

        {/* SR-only status */}
        <span className="ws-sr-status" role="status" aria-live="polite">
          {loading ? 'Loading workspaces...' : error && !loading ? 'Connection error' : `${sortedWorkspaces.length} workspaces available`}
        </span>

        {/* Layer 5: Overlays */}
        <LogoutModal
          open={showLogoutModal}
          onCancel={handleLogoutCancel}
          onConfirm={handleLogoutConfirm}
        />
      </div>
    );
  }

  // ── Error state (no fallback available) ──────────────────────

  if (error && availableWorkspaces.length === 0) {
    return (
      <div className="workspace-home">
        {/* Layer 1: Background */}
        <LayerBackground />

        {/* Layer 2+3: Content container + content */}
        <div className="ws-layer-content">
          <div className="ws-header">
            <LayerFloatingButtons {...floatingProps} />
          </div>
          <div className="ws-main">
            <div className="workspace-error">
              <div className="workspace-error-icon" aria-hidden="true">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
                  <circle cx="12" cy="12" r="10" />
                  <line x1="12" y1="8" x2="12" y2="12" />
                  <line x1="12" y1="16" x2="12.01" y2="16" />
                </svg>
              </div>
              <p className="workspace-error-title">
                <Localized id="workspace-home-error-title">
                  <span>Connection Error</span>
                </Localized>
              </p>
              <p className="workspace-error-desc">
                <Localized id="workspace-home-error-desc">
                  <span>Could not load your workspaces. Check your connection and try again.</span>
                </Localized>
              </p>
              <button
                type="button"
                className="workspace-error-retry"
                onClick={retry}
              >
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                  <polyline points="1 4 1 10 7 10" />
                  <path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10" />
                </svg>
                <Localized id="workspace-home-retry">
                  <span>Try Again</span>
                </Localized>
              </button>
            </div>
          </div>
          <div className="ws-footer" />
        </div>

        {/* Layer 5: Overlays */}
        <LogoutModal
          open={showLogoutModal}
          onCancel={handleLogoutCancel}
          onConfirm={handleLogoutConfirm}
        />
      </div>
    );
  }

  // ── Main render ─────────────────────────────────────────────

  return (
    <div className="workspace-home">
      {/* Layer 1: Background */}
      <LayerBackground />

      {/* Layer 2+3: Content container + content */}
      <div className="ws-layer-content">
        <div className="ws-header">
          <LayerFloatingButtons {...floatingProps} />
        </div>
        <div className="ws-main">
          <header className="workspace-home-header" />

          {sortedWorkspaces.length === 0 ? (
            <div className="workspace-empty">
              <div className="workspace-empty-icon" aria-hidden="true">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
                  <rect x="2" y="3" width="20" height="14" rx="2" ry="2" />
                  <line x1="8" y1="21" x2="16" y2="21" />
                  <line x1="12" y1="17" x2="12" y2="21" />
                  <line x1="7" y1="9" x2="17" y2="9" />
                </svg>
              </div>
              <p className="workspace-empty-title">
                <Localized id="workspace-home-empty">
                  <span>No workspaces available</span>
                </Localized>
              </p>
              <p className="workspace-empty-desc">
                <Localized id="workspace-home-empty-desc">
                  <span>You don&apos;t have access to any workspaces yet. Contact an administrator.</span>
                </Localized>
              </p>
            </div>
          ) : (
            <div className="workspace-grid" ref={gridRef} role="group" aria-label="Workspaces">
              {sortedWorkspaces.map((ws, idx) => {
                const disabled = !canAccess(ws.type_key);
                const colorClass = WS_COLORS[ws.type_key] ?? '';
                const isActive = ws.type_key === lastWorkspace && !disabled;
                if (disabled) {
                  return (
                    <div
                      key={ws.type_key}
                      className={`workspace-card ${colorClass} workspace-card--disabled`}
                      aria-label={l10n.getString('workspace-card-no-access-aria', { name: ws.name })}
                    >
                      <div className="workspace-card-key-hint">{idx + 1}</div>
                      <div className="workspace-card-row">
                        <div className="workspace-card-icon">
                          <div className="workspace-card-icon-inner">
                            {getIcon(ws.type_key)}
                          </div>
                        </div>
                        <div className="workspace-card-body">
                          <div className="workspace-card-title">
                            <h2 className="workspace-card-name">{ws.name}</h2>
                          </div>
                          <div className="workspace-card-text">
                            <p className="workspace-card-desc">{ws.description}</p>
                          </div>
                          <div className="workspace-card-actions">
                            <span className="workspace-card-badge">
                              <Localized id="workspace-card-no-access-badge">
                                <span>Not available</span>
                              </Localized>
                            </span>
                          </div>
                        </div>
                      </div>
                    </div>
                  );
                }

                return (
                  <button
                    key={ws.type_key}
                    type="button"
                    aria-current={isActive ? 'true' : undefined}
                    className={`workspace-card ${colorClass}${isActive ? ' workspace-card--active' : ''}${exitingWorkspace === ws.type_key ? ' workspace-card--exiting' : ''}`}
                    onClick={(e) => handleCardClick(ws.type_key, e)}
                    disabled={exitingWorkspace !== null}
                    onMouseMove={handleMouseMove}
                    onMouseLeave={handleMouseLeave}
                    aria-label={l10n.getString('workspace-card-open-aria', { name: ws.name })}
                  >
                    <div className="workspace-card-key-hint">{idx + 1}</div>
                    {isActive && (
                      <div className="workspace-card-active-dot" aria-label="Active workspace">
                        <svg viewBox="0 0 24 24" fill="currentColor" width="10" height="10" aria-hidden="true">
                          <circle cx="12" cy="12" r="6" />
                        </svg>
                      </div>
                    )}
                    <div className="workspace-card-row">
                      <div className="workspace-card-icon">
                        <div className="workspace-card-icon-inner">
                          {getIcon(ws.type_key)}
                        </div>
                      </div>
                      <div className="workspace-card-body">
                        <div className="workspace-card-title">
                          <h2 className="workspace-card-name">{ws.name}</h2>
                        </div>
                        <div className="workspace-card-text">
                          <p className="workspace-card-desc">{ws.description}</p>
                        </div>
                        <div className="workspace-card-actions" />
                      </div>
                    </div>

                    {/* Overlay: keyboard shortcut hint */}
                    <div className="workspace-card-overlay" aria-hidden="true">
                      <span className="workspace-card-overlay-hint">
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" width="12" height="12">
                          <rect x="2" y="4" width="20" height="16" rx="2" />
                          <path d="M6 8h.01M10 8h.01M14 8h.01M18 8h.01" />
                          <path d="M6 12h.01M10 12h.01M14 12h.01M18 12h.01" />
                        </svg>
                        <Localized id="workspace-home-shortcut-hint" vars={{ key: `${idx + 1}` }}>
                          <span>Press {idx + 1} to open</span>
                        </Localized>
                      </span>
                    </div>
                  </button>
                );
              })}
              {COMING_SOON_CARDS.map((cs) => (
                <div
                  key={cs.name}
                  className="workspace-card workspace-card--disabled"
                >
                  <div className="workspace-card-row">
                    <div className="workspace-card-icon">
                      <div className="workspace-card-icon-inner">
                        {getIcon(cs.name)}
                      </div>
                    </div>
                    <div className="workspace-card-body">
                      <div className="workspace-card-title">
                        <h2 className="workspace-card-name">{cs.name}</h2>
                      </div>
                      <div className="workspace-card-text">
                        <p className="workspace-card-desc">{cs.description}</p>
                      </div>
                      <div className="workspace-card-actions">
                        <span className="workspace-card-badge">
                          <Localized id="workspace-card-no-access-badge">
                            <span>Not available</span>
                          </Localized>
                        </span>
                      </div>
                    </div>
                  </div>

                  {/* Overlay: coming soon hint */}
                  <div className="workspace-card-overlay" aria-hidden="true">
                    <span className="workspace-card-overlay-hint">
                      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" width="12" height="12">
                        <circle cx="12" cy="12" r="10" />
                        <polyline points="12 6 12 12 16 14" />
                      </svg>
                      Coming soon
                    </span>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
        <div className="ws-footer" />
      </div>

      {/* Layer 5: Overlays */}
      <LogoutModal
        open={showLogoutModal}
        onCancel={handleLogoutCancel}
        onConfirm={handleLogoutConfirm}
      />
    </div>
  );
}
