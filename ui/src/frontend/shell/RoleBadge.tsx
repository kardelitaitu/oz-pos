import { useAuth } from '@/contexts/AuthContext';
import './RoleBadge.css';

/**
 * A compact badge showing the active user's display name and role.
 *
 * Rendered in the AppLayout sidebar header. Shows a person icon,
 * the staff member's name, and a small role chip underneath.
 * Clicking the badge opens a logout confirmation.
 */
export default function RoleBadge() {
  const { session, logout } = useAuth();

  if (!session) return null;

  // Map role_name to a display variant.
  const roleVariant = (): 'owner' | 'manager' | 'cashier' => {
    switch (session.role_name) {
      case 'owner': return 'owner';
      case 'manager': return 'manager';
      default: return 'cashier';
    }
  };

  const variant = roleVariant();

  return (
    <div className="role-badge" aria-label={`Logged in as ${session.display_name}, ${session.role_name}`}>
      <div className="role-badge-avatar">
        {session.display_name.charAt(0).toUpperCase()}
      </div>
      <div className="role-badge-info">
        <span className="role-badge-name">{session.display_name}</span>
        <span className={`role-badge-role role-badge-role--${variant}`}>
          {session.role_name}
        </span>
      </div>
      <button
        type="button"
        className="role-badge-logout"
        onClick={logout}
        aria-label={`Log out ${session.display_name}`}
        title="Log out"
      >
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="16" height="16" aria-hidden="true">
          <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" />
          <polyline points="16 17 21 12 16 7" />
          <line x1="21" y1="12" x2="9" y2="12" />
        </svg>
      </button>
    </div>
  );
}
