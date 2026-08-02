/**
 * Empty State Illustrations — inline SVG components for data-free screens.
 *
 * Each illustration is a 48×48 viewBox with currentColor stroke/fill so
 * they adapt to the active theme (light/dark). Use them as the `icon` prop
 * of the `<EmptyState>` component.
 *
 * EMPTY-09: `emptyStateIconFor(resource)` provides a small mapping from
 * common resource types to the matching illustration so no-data screens
 * share one visual language. Add new resource icons here (currentColor,
 * aria-hidden) and extend the mapping rather than authoring inline SVGs in
 * feature screens.
 */

/** Props shared by all illustration components. */
export interface IlluProps {
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

/** Folder/tag icon — for "no categories" empty states. */
export function NoCategoriesIcon({ width = 48, height = 48 }: IlluProps) {
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
      <path d="M6 12a2 2 0 0 1 2-2h10l4 4h18a2 2 0 0 1 2 2v22a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2z" opacity="0.4" />
      <path d="M6 16h36" opacity="0.5" />
      <path d="M18 26l6-6 6 6" opacity="0.6" />
      <line x1="30" y1="26" x2="34" y2="26" opacity="0.4" />
      <circle cx="40" cy="10" r="5" opacity="0.35" strokeDasharray="2 2" />
    </svg>
  );
}

/** Gift-card icon — for "no gift cards" empty states. */
export function NoGiftCardsIcon({ width = 48, height = 48 }: IlluProps) {
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
      <rect x="6" y="16" width="36" height="24" rx="3" opacity="0.4" />
      <rect x="8" y="18" width="32" height="20" rx="2" />
      <line x1="6" y1="24" x2="42" y2="24" opacity="0.4" />
      <path d="M24 18v10" opacity="0.5" />
      <path d="M24 24l-5-5a3.5 3.5 0 0 1 5-5c1.2 1.2 1.2 3.8 0 5z" opacity="0.4" />
      <path d="M24 24l5-5a3.5 3.5 0 0 0-5-5c-1.2 1.2-1.2 3.8 0 5z" opacity="0.4" />
      <line x1="14" y1="30" x2="20" y2="30" opacity="0.5" />
      <line x1="28" y1="30" x2="34" y2="30" opacity="0.5" />
      <line x1="14" y1="34" x2="20" y2="34" opacity="0.4" />
      <line x1="28" y1="34" x2="34" y2="34" opacity="0.4" />
    </svg>
  );
}

/** Truck icon — for "no suppliers" empty states. */
export function NoSuppliersIcon({ width = 48, height = 48 }: IlluProps) {
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
      <path d="M6 12h22v20H6z" opacity="0.4" />
      <path d="M16 12h14v8h8l6 6v6h-6" opacity="0.5" />
      <path d="M28 20h10l6 6v6" opacity="0.5" />
      <circle cx="14" cy="36" r="4" opacity="0.6" />
      <circle cx="36" cy="36" r="4" opacity="0.6" />
      <line x1="14" y1="20" x2="22" y2="20" opacity="0.4" />
      <line x1="14" y1="24" x2="22" y2="24" opacity="0.4" />
      <circle cx="44" cy="8" r="4" opacity="0.35" strokeDasharray="2 2" />
    </svg>
  );
}

/** Document icon — for "no purchase orders" empty states. */
export function NoPurchaseOrdersIcon({ width = 48, height = 48 }: IlluProps) {
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
      <path d="M12 6h16l8 8v28H12z" opacity="0.4" />
      <path d="M28 6v8h8" opacity="0.5" />
      <line x1="16" y1="20" x2="32" y2="20" opacity="0.5" />
      <line x1="16" y1="24" x2="32" y2="24" opacity="0.4" />
      <line x1="16" y1="28" x2="26" y2="28" opacity="0.5" />
      <line x1="16" y1="32" x2="30" y2="32" opacity="0.4" />
      <circle cx="38" cy="38" r="6" opacity="0.35" strokeDasharray="2 2" />
    </svg>
  );
}

/** Tag icon — for "no variants" empty states. */
export function NoVariantsIcon({ width = 48, height = 48 }: IlluProps) {
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
      <path d="M8 6h24a2 2 0 0 1 2 2v14l-18 18a2 2 0 0 1-3 0L7 25a2 2 0 0 1 0-3L25 6z" opacity="0.4" />
      <path d="M14 14h14" opacity="0.5" />
      <circle cx="32" cy="16" r="4" opacity="0.6" />
      <line x1="32" y1="14" x2="32" y2="18" opacity="0.7" />
      <circle cx="42" cy="8" r="4" opacity="0.35" strokeDasharray="2 2" />
    </svg>
  );
}

/** Percent/ticket icon — for "no promotions" empty states. */
export function NoPromotionsIcon({ width = 48, height = 48 }: IlluProps) {
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
      <path d="M8 24l16-16 14 14-16 16z" opacity="0.4" />
      <path d="M8 24l4-4 16 16-4 4z" opacity="0.3" />
      <line x1="14" y1="16" x2="18" y2="12" opacity="0.5" />
      <circle cx="20" cy="22" r="2" opacity="0.6" />
      <circle cx="28" cy="30" r="2" opacity="0.6" />
      <line x1="14" y1="34" x2="30" y2="18" opacity="0.5" />
      <circle cx="40" cy="10" r="4" opacity="0.35" strokeDasharray="2 2" />
    </svg>
  );
}

/** Star-card icon — for "no loyalty accounts" empty states. */
export function NoLoyaltyIcon({ width = 48, height = 48 }: IlluProps) {
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
      <rect x="6" y="14" width="36" height="26" rx="3" opacity="0.4" />
      <rect x="8" y="16" width="32" height="22" rx="2" />
      <path d="M24 21l2 4 4.5.6-3.3 3.2.8 4.5L24 31.5l-4 2.3.8-4.5-3.3-3.2L22 25z" opacity="0.6" />
      <line x1="12" y1="32" x2="18" y2="32" opacity="0.4" />
      <circle cx="38" cy="10" r="4" opacity="0.35" strokeDasharray="2 2" />
    </svg>
  );
}

/** Device/terminal icon — for "no terminals" empty states. */
export function NoTerminalsIcon({ width = 48, height = 48 }: IlluProps) {
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
      <rect x="6" y="6" width="36" height="26" rx="3" opacity="0.4" />
      <rect x="8" y="8" width="32" height="22" rx="2" />
      <line x1="18" y1="40" x2="30" y2="40" opacity="0.5" />
      <line x1="24" y1="32" x2="24" y2="40" opacity="0.5" />
      <line x1="14" y1="14" x2="20" y2="18" opacity="0.6" />
      <line x1="20" y1="14" x2="14" y2="18" opacity="0.6" />
      <line x1="26" y1="14" x2="30" y2="14" opacity="0.4" />
      <circle cx="40" cy="40" r="4" opacity="0.35" strokeDasharray="2 2" />
    </svg>
  );
}

/** People icon — for "no customers" empty states (distinct from staff). */
export function NoCustomersIcon({ width = 48, height = 48 }: IlluProps) {
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
      <circle cx="18" cy="14" r="6" opacity="0.6" />
      <path d="M8 38c0-5.523 4.477-10 10-10s10 4.477 10 10" opacity="0.5" />
      <path d="M30 10c3.314 0 6 2.686 6 6s-2.686 6-6 6" opacity="0.35" />
      <path d="M28 28.2c3.6-1.2 7.6.2 9.6 3.4.9 1.4 1.4 3 1.4 4.6" opacity="0.3" />
      <circle cx="40" cy="40" r="4" opacity="0.35" strokeDasharray="2 2" />
    </svg>
  );
}

// ── Resource → icon mapping (EMPTY-09) ─────────────────────────────

/** Resource types with a dedicated empty-state illustration. */
export type EmptyStateResource =
  | 'products'
  | 'sales'
  | 'staff'
  | 'shifts'
  | 'categories'
  | 'customers'
  | 'gift-cards'
  | 'suppliers'
  | 'purchase-orders'
  | 'variants'
  | 'promotions'
  | 'loyalty'
  | 'terminals'
  | 'search'
  | 'generic';

/** Small mapping from common resource types to their illustration. */
export const EMPTY_STATE_RESOURCE_ICONS: Record<EmptyStateResource, (props: IlluProps) => React.JSX.Element> = {
  products: NoProductsIcon,
  sales: NoSalesIcon,
  staff: NoStaffIcon,
  shifts: NoShiftsIcon,
  categories: NoCategoriesIcon,
  customers: NoCustomersIcon,
  'gift-cards': NoGiftCardsIcon,
  suppliers: NoSuppliersIcon,
  'purchase-orders': NoPurchaseOrdersIcon,
  variants: NoVariantsIcon,
  promotions: NoPromotionsIcon,
  loyalty: NoLoyaltyIcon,
  terminals: NoTerminalsIcon,
  search: NotFoundIcon,
  generic: EmptyBoxIcon,
};

/** Returns the illustration component for a resource type (fallback: generic box). */
export function emptyStateIconFor(resource: EmptyStateResource | string): (props: IlluProps) => React.JSX.Element {
  return EMPTY_STATE_RESOURCE_ICONS[resource as EmptyStateResource] ?? EmptyBoxIcon;
}
