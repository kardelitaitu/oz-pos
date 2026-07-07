import { useCallback, useMemo } from 'react';
import { useWorkspace } from '@/contexts/WorkspaceContext';
import { useAuth } from '@/contexts/AuthContext';
import { Localized, useLocalization } from '@fluent/react';
import './WorkspaceHome.css';

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
    default:
      console.warn(`WorkspaceHome: unknown workspace key "${key}" — using default icon`);
      return (
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
          <circle cx="12" cy="12" r="10" />
        </svg>
      );
  }
}

export default function WorkspaceHome() {
  const { l10n } = useLocalization();
  const { availableWorkspaces, loading, setActiveWorkspace } = useWorkspace();
  const { session, logout } = useAuth();

  const roleName = session?.role_name ?? '';

  const cashierOnly = useMemo(() => new Set(['restaurant-pos', 'store-pos']), []);

  const canAccess = useCallback(
    (key: string) => roleName === 'owner' || roleName === 'manager' || cashierOnly.has(key),
    [roleName, cashierOnly],
  );

  const handleSelect = useCallback((key: string) => {
    if (!canAccess(key)) return;
    setActiveWorkspace(key);
  }, [canAccess, setActiveWorkspace]);

  if (loading) {
    return (
      <div className="workspace-home">
        <div className="workspace-loading">
          <Localized id="workspace-home-loading"><span>Loading workspaces…</span></Localized>
        </div>
      </div>
    );
  }

  return (
    <div className="workspace-home">
      <div className="workspace-home-top-bar">
        <button type="button" className="workspace-home-logout-btn" onClick={logout}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" width="20" height="20">
            <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" />
            <polyline points="16 17 21 12 16 7" />
            <line x1="21" y1="12" x2="9" y2="12" />
          </svg>
          <Localized id="workspace-home-logout"><span>Logout</span></Localized>
        </button>
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
        <div className="workspace-grid">
          {availableWorkspaces.map((ws) => {
            const disabled = !canAccess(ws.key);
            return (
              <button
                key={ws.key}
                type="button"
                className={`workspace-card${disabled ? ' workspace-card--disabled' : ''}`}
                onClick={() => handleSelect(ws.key)}
                disabled={disabled}
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
