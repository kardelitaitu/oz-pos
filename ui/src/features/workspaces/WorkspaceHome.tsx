import { useCallback, useMemo, useRef, useEffect } from 'react';
import { useWorkspace } from '@/contexts/WorkspaceContext';
import { useAuth } from '@/contexts/AuthContext';
import { Localized, useLocalization } from '@fluent/react';
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
  switch (key) {
    case 'restaurant-pos':
      return (
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
          <path d="M3 3h18v18H3z" />
          <path d="M12 8v8M8 12h8" />
        </svg>
      );
    case 'store-pos':
      return (
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
          <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
          <polyline points="9 22 9 12 15 12 15 22" />
        </svg>
      );
    case 'inventory':
      return (
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
          <path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2" />
          <rect x="8" y="2" width="8" height="4" rx="1" ry="1" />
          <line x1="8" y1="12" x2="16" y2="12" />
          <line x1="8" y1="16" x2="14" y2="16" />
        </svg>
      );
    case 'admin':
      return (
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
          <circle cx="12" cy="12" r="3" />
          <path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42" />
        </svg>
      );
    case 'kds':
      return (
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
          <rect x="2" y="3" width="20" height="14" rx="2" ry="2" />
          <line x1="8" y1="21" x2="16" y2="21" />
          <line x1="12" y1="17" x2="12" y2="21" />
          <path d="M7 9l3 3-3 3" />
          <path d="M17 9l-3 3 3 3" />
          <circle cx="12" cy="12" r="1" fill="currentColor" />
        </svg>
      );
    default:
      console.warn(`WorkspaceHome: unknown workspace key "${key}" — using default icon`);
      return (
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
          <circle cx="12" cy="12" r="10" />
        </svg>
      );
  }
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

// ── Component ─────────────────────────────────────────────────────

export default function WorkspaceHome() {
  const { l10n } = useLocalization();
  const { availableWorkspaces, loading, error, retry, setActiveWorkspace } = useWorkspace();
  const { session, logout } = useAuth();
  const gridRef = useRef<HTMLDivElement>(null);

  const roleName = (session?.role_name ?? '').toLowerCase();

  const cashierOnly = useMemo(() => new Set(['restaurant-pos', 'store-pos']), []);
  const kitchenOnly = useMemo(() => new Set(['kds']), []);

  const canAccess = useCallback(
    (key: string) =>
      roleName === 'owner' ||
      roleName === 'manager' ||
      cashierOnly.has(key) ||
      (roleName === 'kitchen' && kitchenOnly.has(key)),
    [roleName, cashierOnly, kitchenOnly],
  );

  const handleSelect = useCallback((key: string) => {
    if (!canAccess(key)) return;
    setActiveWorkspace(key);
  }, [canAccess, setActiveWorkspace]);

  // ── Keyboard navigation ──────────────────────────────────────

  useEffect(() => {
    const grid = gridRef.current;
    if (!grid) return;

    const cards = grid.querySelectorAll<HTMLButtonElement>('.workspace-card');
    if (cards.length === 0) return;

    function focusCard(index: number) {
      const target = cards[index];
      if (target && !target.disabled) {
        target.focus();
      }
    }

    function getColumns(): number {
      // Compute columns from grid layout
      const style = getComputedStyle(grid!);
      const gridCols = style.gridTemplateColumns.split(' ');
      return gridCols.length;
    }

    const handleKeyDown = (e: KeyboardEvent) => {
      const active = document.activeElement;
      if (!active || !grid.contains(active)) return;

      // Find the index of the currently focused card
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
  }, [availableWorkspaces]);

  // ── Mousemove glow effect ────────────────────────────────────

  const handleMouseMove = useCallback((e: React.MouseEvent<HTMLButtonElement>) => {
    const card = e.currentTarget;
    const rect = card.getBoundingClientRect();
    const x = ((e.clientX - rect.left) / rect.width) * 100;
    const y = ((e.clientY - rect.top) / rect.height) * 100;
    card.style.setProperty('--mouse-x', `${x}%`);
    card.style.setProperty('--mouse-y', `${y}%`);
  }, []);

  // ── Loading state ────────────────────────────────────────────

  if (loading) {
    return (
      <div className="workspace-home">
        <div className="workspace-home-top-bar">
          {session && (
            <button type="button" className="workspace-home-logout-btn" onClick={logout}>
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" width="20" height="20">
                <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" />
                <polyline points="16 17 21 12 16 7" />
                <line x1="21" y1="12" x2="9" y2="12" />
              </svg>
              <Localized id="workspace-home-logout"><span>Logout</span></Localized>
            </button>
          )}
        </div>
        <header className="workspace-home-header">
          <h1 className="workspace-home-title">OZ-POS</h1>
          <p className="workspace-home-subtitle">
            <Localized id="workspace-home-subtitle">
              <span>Select a workspace to start</span>
            </Localized>
          </p>
        </header>
        <SkeletonGrid />
      </div>
    );
  }

  // ── Error state (no fallback available) ──────────────────────

  if (error && availableWorkspaces.length === 0) {
    return (
      <div className="workspace-home">
        <div className="workspace-home-top-bar">
          {session && (
            <button type="button" className="workspace-home-logout-btn" onClick={logout}>
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" width="20" height="20">
                <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" />
                <polyline points="16 17 21 12 16 7" />
                <line x1="21" y1="12" x2="9" y2="12" />
              </svg>
              <Localized id="workspace-home-logout"><span>Logout</span></Localized>
            </button>
          )}
        </div>
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
    );
  }

  // ── Main render ─────────────────────────────────────────────

  return (
    <div className="workspace-home">
      <div className="workspace-home-top-bar">
        {session && (
          <button type="button" className="workspace-home-logout-btn" onClick={logout}>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" width="20" height="20">
              <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" />
              <polyline points="16 17 21 12 16 7" />
              <line x1="21" y1="12" x2="9" y2="12" />
            </svg>
            <Localized id="workspace-home-logout"><span>Logout</span></Localized>
          </button>
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
      <header className="workspace-home-header">
        <h1 className="workspace-home-title">OZ-POS</h1>
        <p className="workspace-home-subtitle">
          <Localized id="workspace-home-subtitle">
            <span>Select a workspace to start</span>
          </Localized>
        </p>
        {session && (
          <span className="workspace-home-user">
            {session.display_name} ({session.role_name})
          </span>
        )}
      </header>

      {availableWorkspaces.length === 0 ? (
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
        <div className="workspace-grid" ref={gridRef} role="listbox" aria-label="Workspaces">
          {[...availableWorkspaces]
            .sort((a, b) => (WS_ORDER[a.key] ?? 99) - (WS_ORDER[b.key] ?? 99))
            .map((ws) => {
            const disabled = !canAccess(ws.key);
            const colorClass = WS_COLORS[ws.key] ?? '';
            return (
              <button
                key={ws.key}
                type="button"
                role="option"
                aria-selected={false}
                className={`workspace-card ${colorClass}${disabled ? ' workspace-card--disabled' : ''}`}
                onClick={() => handleSelect(ws.key)}
                disabled={disabled}
                onMouseMove={handleMouseMove}
                aria-label={l10n.getString(
                  disabled ? 'workspace-card-no-access-aria' : 'workspace-card-open-aria',
                  { name: ws.name },
                )}
                title={disabled ? l10n.getString('workspace-card-no-access-title', { role: roleName }) : ws.name}
              >
                <div className="workspace-card-icon">
                  {getIcon(ws.key)}
                </div>
                <div className="workspace-card-body">
                  <h2 className="workspace-card-name">{ws.name}</h2>
                  <p className="workspace-card-desc">{ws.description}</p>
                  {disabled && (
                    <span className="workspace-card-badge">
                      <Localized id="workspace-card-no-access-badge">
                        <span>Not available</span>
                      </Localized>
                    </span>
                  )}
                </div>
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}
