import { invoke } from '@tauri-apps/api/core';

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

/** List all product bundles. */
export const listBundles = (): Promise<BundleWithItems[]> =>
  invoke<BundleWithItems[]>('list_bundles');

/** Get a single bundle by its identifier. */
export const getBundle = (id: string): Promise<BundleWithItems | null> =>
  invoke<BundleWithItems | null>('get_bundle', { id });

/** Create a new product bundle. */
export const createBundle = (args: CreateBundleArgs): Promise<BundleWithItems> =>
  invoke<BundleWithItems>('create_bundle', { args });

/** Update an existing product bundle. */
export const updateBundle = (bundle: BundleWithItems): Promise<BundleWithItems> =>
  invoke<BundleWithItems>('update_bundle', { bundle });

/** Delete a product bundle by its identifier. */
export const deleteBundle = (id: string): Promise<void> =>
  invoke<void>('delete_bundle', { id });

/** Look up a bundle by its SKU. */
export const lookupBundleBySku = (sku: string): Promise<BundleWithItems | null> =>
  invoke<BundleWithItems | null>('lookup_bundle_by_sku', { sku });
