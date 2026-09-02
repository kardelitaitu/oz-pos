import { loggedInvoke } from '@/utils/logged-invoke';

/** A product bundle definition. */
export interface ProductBundle {
  id: string;
  bundle_sku: string;
  name: string;
  description: string;
  bundle_price_minor: number | null;
  currency: string;
  active: boolean;
  created_at: string;
  updated_at: string;
}

/** An item (product) within a bundle. */
export interface BundleItem {
  id: string;
  bundle_id: string;
  sku: string;
  qty: number;
  unit_price_minor: number | null;
}

/** A bundle with its resolved items. */
export interface BundleWithItems {
  bundle: ProductBundle;
  items: BundleItem[];
}

/** Arguments for creating a new bundle. */
export interface CreateBundleArgs {
  bundle_sku: string;
  name: string;
  description?: string;
  bundle_price_minor?: number | null;
  currency?: string;
  items: { sku: string; qty: number; unit_price_minor?: number | null }[];
}

/** List all product bundles (session-scoped). */
export const listBundles = (sessionToken: string): Promise<BundleWithItems[]> =>
  loggedInvoke<BundleWithItems[]>('list_bundles_scoped', { sessionToken });

/** Get a single bundle by its identifier (session-scoped). */
export const getBundle = (sessionToken: string, id: string): Promise<BundleWithItems | null> =>
  loggedInvoke<BundleWithItems | null>('get_bundle_scoped', { sessionToken, id });

/** Create a new product bundle (session-scoped). */
export const createBundle = (sessionToken: string, args: CreateBundleArgs): Promise<BundleWithItems> =>
  loggedInvoke<BundleWithItems>('create_bundle_scoped', { sessionToken, args });

/** Update an existing product bundle (session-scoped). */
export const updateBundle = (sessionToken: string, bundle: BundleWithItems): Promise<BundleWithItems> =>
  loggedInvoke<BundleWithItems>('update_bundle_scoped', { sessionToken, bundle });

/** Delete a product bundle by its identifier (session-scoped). */
export const deleteBundle = (sessionToken: string, id: string): Promise<void> =>
  loggedInvoke<void>('delete_bundle_scoped', { sessionToken, id });

/** Look up a bundle by its SKU (session-scoped). */
export const lookupBundleBySku = (sessionToken: string, sku: string): Promise<BundleWithItems | null> =>
  loggedInvoke<BundleWithItems | null>('lookup_bundle_by_sku_scoped', { sessionToken, sku });
