import { invoke } from '@tauri-apps/api/core';

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

export interface BundleItem {
  id: string;
  bundle_id: string;
  sku: string;
  qty: number;
  unit_price_minor: number | null;
}

export interface BundleWithItems {
  bundle: ProductBundle;
  items: BundleItem[];
}

export interface CreateBundleArgs {
  bundle_sku: string;
  name: string;
  description?: string;
  bundle_price_minor?: number | null;
  currency?: string;
  items: { sku: string; qty: number; unit_price_minor?: number | null }[];
}

export const listBundles = (): Promise<BundleWithItems[]> =>
  invoke<BundleWithItems[]>('list_bundles');

export const getBundle = (id: string): Promise<BundleWithItems | null> =>
  invoke<BundleWithItems | null>('get_bundle', { id });

export const createBundle = (args: CreateBundleArgs): Promise<BundleWithItems> =>
  invoke<BundleWithItems>('create_bundle', { args });

export const updateBundle = (bundle: BundleWithItems): Promise<BundleWithItems> =>
  invoke<BundleWithItems>('update_bundle', { bundle });

export const deleteBundle = (id: string): Promise<void> =>
  invoke<void>('delete_bundle', { id });

export const lookupBundleBySku = (sku: string): Promise<BundleWithItems | null> =>
  invoke<BundleWithItems | null>('lookup_bundle_by_sku', { sku });
