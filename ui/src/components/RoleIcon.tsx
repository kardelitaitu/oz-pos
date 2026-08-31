/*
last audited 31-08-26 by RSA-Agent (user-role campaign, Section H)
findings: display drift — doc comment promises "admin: Shield" but the switch has NO admin case (normalizeRole yields admin, falls to default staff icon); auditor/owner/manager correct
next: batch into the fix-order phase | perf: n/a (presentational)
*/
import { normalizeRole } from '@/utils/role';

interface RoleIconProps {
  role?: string | null;
  className?: string;
  size?: number;
}

/**
 * Returns a dedicated SVG icon component for a given role.
 * - owner: Crown
 * - admin: Shield
 * - manager: Briefcase / Shield
 * - auditor: Eye (read-only)
 * - staff: ID Card / Staff Badge (Default)
 */
export function RoleIcon({ role, className = '', size = 16 }: RoleIconProps) {
  const variant = normalizeRole(role);

  switch (variant) {
    case 'owner':
      return (
        <svg
          viewBox="0 0 24 24"
          width={size}
          height={size}
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          className={className}
          aria-hidden="true"
        >
          <path d="M11.562 3.266a.5.5 0 0 1 .876 0L15.39 8.87a1 1 0 0 0 1.516.294L21.183 5.5a.5.5 0 0 1 .798.519l-2.834 10.24a1 1 0 0 1-.964.733H4.82a1 1 0 0 1-.964-.733L1.02 6.02a.5.5 0 0 1 .798-.519l4.276 3.664a1 1 0 0 0 1.516-.294z" />
          <path d="M5 21h14" />
        </svg>
      );

    case 'admin':
      // H-2: the doc contract promises "admin: Shield" — render it instead
      // of silently falling through to the staff default.
      return (
        <svg
          viewBox="0 0 24 24"
          width={size}
          height={size}
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          className={className}
          aria-hidden="true"
        >
          <path d="M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z" />
        </svg>
      );

    case 'manager':
      return (
        <svg
          viewBox="0 0 24 24"
          width={size}
          height={size}
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          className={className}
          aria-hidden="true"
        >
          <rect x="2" y="7" width="20" height="14" rx="2" ry="2" />
          <path d="M16 21V5a2 2 0 0 0-2-2h-4a2 2 0 0 0-2 2v16" />
        </svg>
      );

    case 'auditor':
      return (
        <svg
          viewBox="0 0 24 24"
          width={size}
          height={size}
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          className={className}
          aria-hidden="true"
        >
          <path d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7-10-7-10-7z" />
          <circle cx="12" cy="12" r="3" />
        </svg>
      );

    case 'staff':
    default:
      return (
        <svg
          viewBox="0 0 24 24"
          width={size}
          height={size}
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          className={className}
          aria-hidden="true"
        >
          <rect x="3" y="4" width="18" height="18" rx="2" ry="2" />
          <circle cx="12" cy="10" r="3" />
          <path d="M7 17v-1a5 5 0 0 1 10 0v1" />
        </svg>
      );
  }
}
