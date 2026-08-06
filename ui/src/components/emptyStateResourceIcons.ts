/**
 * Empty-state resource icon mapping (EMPTY-09).
 *
 * Separate module so `EmptyStateIllustrations.tsx` stays a
 * components-only file (react-refresh/only-export-components): the
 * icons are components; the mapping is a constant + function.
 */

import {
  NoProductsIcon,
  NoSalesIcon,
  NoStaffIcon,
  NoShiftsIcon,
  NoCategoriesIcon,
  NoCustomersIcon,
  NoGiftCardsIcon,
  NoSuppliersIcon,
  NoPurchaseOrdersIcon,
  NoVariantsIcon,
  NoPromotionsIcon,
  NoLoyaltyIcon,
  NoTerminalsIcon,
  NotFoundIcon,
  EmptyBoxIcon,
  type IlluProps,
} from './EmptyStateIllustrations';

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
