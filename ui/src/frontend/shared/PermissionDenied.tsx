import { Localized } from '@fluent/react';
import { useAuth } from '@/contexts/AuthContext';
import './PermissionDenied.css';

interface PermissionDeniedProps {
  /** What action was denied (e.g. "void orders", "edit settings"). */
  action: string;
  /** The minimum role required (e.g. "Manager" or "Owner"). */
  requiredRole: string;
  /** Optional: called when the user dismisses the screen. */
  onDismiss?: () => void;
}

/**
 * Friendly error screen shown when a cashier tries a manager-only action.
 */
export default function PermissionDenied({ action, requiredRole, onDismiss }: PermissionDeniedProps) {
  const { session } = useAuth();

  return (
    <div className="permission-denied">
      <div className="permission-denied-card">
        <div className="permission-denied-icon" aria-hidden="true">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" width="48" height="48">
            <rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
            <path d="M7 11V7a5 5 0 0 1 10 0v4" />
          </svg>
        </div>

        <h2 className="permission-denied-title">
          <Localized id="permission-denied-title">Access Denied</Localized>
        </h2>

        <p className="permission-denied-desc">
          <Localized id="permission-denied-desc" vars={{ action, requiredRole }}>
            <span><strong>{action}</strong> requires a <strong>{requiredRole}</strong> role.</span>
          </Localized>
        </p>

        {session && (
          <p className="permission-denied-current">
            <Localized id="permission-denied-current" vars={{ displayName: session.display_name, roleName: session.role_name }}>
              <span>You are logged in as <strong>{session.display_name}</strong> ({session.role_name}).</span>
            </Localized>
          </p>
        )}

        {onDismiss && (
          <button
            type="button"
            className="permission-denied-btn"
            onClick={onDismiss}
          >
            <Localized id="permission-denied-go-back">Go back</Localized>
          </button>
        )}
      </div>
    </div>
  );
}
