/**
 * Empty State Illustrations — inline SVG components for data-free screens.
 *
 * Each illustration is a 48×48 viewBox with currentColor stroke/fill so
 * they adapt to the active theme (light/dark). Use them as the `icon` prop
 * of the `<EmptyState>` component.
 */

interface IlluProps {
  width?: number;
  height?: number;
}

/** Box/package icon — for "no products" empty states. */
export function NoProductsIcon({ width = 48, height = 48 }: IlluProps) {
  return (
    <svg
      viewBox="0 0 48 48"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      width={width}
      height={height}
      aria-hidden="true"
      style={{ color: 'var(--color-fg-tertiary)' }}
    >
      {/* Box body */}
      <path d="M12 16L6 19v12l6 3 6-3V19l-6-3z" opacity="0.4" />
      <path d="M18 19l6 3v12l-6-3V19z" />
      <path d="M12 16l6 3-6 3-6-3 6-3z" opacity="0.6" />
      <path d="M24 16l6 3v12l-6-3V19z" opacity="0.4" />
      <path d="M18 19l6 3 6-3" opacity="0.6" />
      {/* Lid */}
      <path d="M6 14l6-3 6 3-6 3-6-3z" opacity="0.3" />
      <path d="M12 11l6 3" opacity="0.5" />
      <path d="M18 14l6-3 6 3" opacity="0.4" />
      {/* Tag / label */}
      <path d="M14 24v4" opacity="0.5" />
      <line x1="14" y1="26" x2="16" y2="26" opacity="0.5" />
      {/* Question mark */}
      <circle cx="38" cy="14" r="6" opacity="0.5" />
      <path d="M38 12v1" opacity="0.7" />
      <path d="M38 15v1" opacity="0.7" />
    </svg>
  );
}

/** Receipt / clipboard icon — for "no sales" empty states. */
export function NoSalesIcon({ width = 48, height = 48 }: IlluProps) {
  return (
    <svg
      viewBox="0 0 48 48"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      width={width}
      height={height}
      aria-hidden="true"
      style={{ color: 'var(--color-fg-tertiary)' }}
    >
      {/* Clipboard */}
      <rect x="12" y="6" width="24" height="36" rx="2" opacity="0.4" />
      <rect x="14" y="8" width="20" height="32" rx="1" />
      {/* Clipboard clip */}
      <path d="M18 6v-2a2 2 0 0 1 2-2h8a2 2 0 0 1 2 2v2" opacity="0.6" />
      {/* Lines of text (receipt items) */}
      <line x1="18" y1="16" x2="30" y2="16" opacity="0.5" />
      <line x1="18" y1="20" x2="26" y2="20" opacity="0.4" />
      <line x1="18" y1="24" x2="28" y2="24" opacity="0.5" />
      <line x1="18" y1="28" x2="24" y2="28" opacity="0.4" />
      {/* Total line */}
      <line x1="18" y1="33" x2="30" y2="33" strokeWidth="2" opacity="0.6" />
      {/* Price marker */}
      <path d="M32 12h4" opacity="0.3" />
      <path d="M32 36h4" opacity="0.3" />
      {/* Empty cart indicator */}
      <circle cx="38" cy="38" r="6" opacity="0.5" strokeDasharray="2 2" />
    </svg>
  );
}

/** People / user-group icon — for "no staff" empty states. */
export function NoStaffIcon({ width = 48, height = 48 }: IlluProps) {
  return (
    <svg
      viewBox="0 0 48 48"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      width={width}
      height={height}
      aria-hidden="true"
      style={{ color: 'var(--color-fg-tertiary)' }}
    >
      {/* Main person */}
      <circle cx="18" cy="14" r="6" opacity="0.6" />
      <path d="M8 38c0-5.523 4.477-10 10-10s10 4.477 10 10" opacity="0.5" />
      {/* Second person (faded) */}
      <circle cx="32" cy="18" r="4" opacity="0.35" />
      <path d="M24 38c0-4.418 3.582-8 8-8s8 3.582 8 8" opacity="0.3" />
      {/* Plus badge */}
      <circle cx="18" cy="14" r="6" opacity="0.4" strokeDasharray="2 2" />
      <line x1="18" y1="11" x2="18" y2="17" opacity="0.4" />
      <line x1="15" y1="14" x2="21" y2="14" opacity="0.4" />
    </svg>
  );
}

/** Calendar icon — for "no shifts" empty states. */
export function NoShiftsIcon({ width = 48, height = 48 }: IlluProps) {
  return (
    <svg
      viewBox="0 0 48 48"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      width={width}
      height={height}
      aria-hidden="true"
      style={{ color: 'var(--color-fg-tertiary)' }}
    >
      {/* Calendar body */}
      <rect x="8" y="12" width="32" height="30" rx="3" opacity="0.4" />
      <rect x="10" y="14" width="28" height="26" rx="2" />
      {/* Header bar */}
      <line x1="10" y1="20" x2="38" y2="20" opacity="0.3" />
      {/* Day grid */}
      <rect x="14" y="24" width="6" height="4" rx="0.5" opacity="0.4" />
      <rect x="22" y="24" width="6" height="4" rx="0.5" opacity="0.3" />
      <rect x="30" y="24" width="6" height="4" rx="0.5" opacity="0.4" />
      <rect x="14" y="30" width="6" height="4" rx="0.5" opacity="0.5" />
      <rect x="22" y="30" width="6" height="4" rx="0.5" opacity="0.3" />
      <rect x="30" y="30" width="6" height="4" rx="0.5" opacity="0.4" />
      {/* Pin / marker on today */}
      <circle cx="17" cy="32" r="2" opacity="0.6" />
      {/* Calendar top rings */}
      <path d="M16 8v4" opacity="0.4" />
      <path d="M32 8v4" opacity="0.4" />
      {/* Empty slot indicator */}
      <circle cx="36" cy="8" r="4" opacity="0.35" strokeDasharray="2 2" />
    </svg>
  );
}

/** Search / magnifying glass icon — for filtered "no results" states. */
export function NotFoundIcon({ width = 48, height = 48 }: IlluProps) {
  return (
    <svg
      viewBox="0 0 48 48"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      width={width}
      height={height}
      aria-hidden="true"
      style={{ color: 'var(--color-fg-tertiary)' }}
    >
      {/* Magnifying glass */}
      <circle cx="20" cy="20" r="12" opacity="0.5" />
      <circle cx="20" cy="20" r="10" />
      <line x1="28" y1="28" x2="36" y2="36" />
      {/* Dash — no results */}
      <line x1="16" y1="24" x2="24" y2="16" opacity="0.5" />
      {/* Faded secondary circle */}
      <circle cx="36" cy="36" r="4" opacity="0.3" strokeDasharray="2 2" />
    </svg>
  );
}

/** Generic empty box icon — fallback for other empty states. */
export function EmptyBoxIcon({ width = 48, height = 48 }: IlluProps) {
  return (
    <svg
      viewBox="0 0 48 48"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      width={width}
      height={height}
      aria-hidden="true"
      style={{ color: 'var(--color-fg-tertiary)' }}
    >
      <path d="M12 4L4 12v30a2 2 0 0 0 2 2h36a2 2 0 0 0 2-2V12l-8-8z" opacity="0.4" />
      <path d="M12 4l-8 8h40l-8-8z" opacity="0.3" />
      <line x1="4" y1="12" x2="44" y2="12" opacity="0.5" />
      <path d="M26 20h-4v8h-8v4h8v8h4v-8h8v-4h-8z" opacity="0.6" />
    </svg>
  );
}
