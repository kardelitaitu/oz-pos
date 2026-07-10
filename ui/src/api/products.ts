// ── Products: CRUD, variants, categories, barcode, stock adjustment ──

import { invoke } from '@tauri-apps/api/core';

// ── Products ──────────────────────────────────────────────────────

export interface ProductDto {
  sku: string;
  name: string;
  category: string | null;
  price: { minor_units: number; currency: string };
  barcode: string | null;
  in_stock: boolean;
  stock_qty: number | null;
  tax_rate_ids: string[];
  created_at: string;
  price_updated_at: string;
  product_type: string;
}

export interface CreateProductArgs {
  userId: string;
  sku: string;
  name: string;
  priceMinor: number;
  currency: string;
  categoryId?: string | undefined;
  barcode?: string | undefined;
  initialStock: number;
  productType?: string;
  taxRateIds: string[];
}

export interface UpdateProductArgs {
  userId: string;
  sku: string;
  name: string;
  priceMinor: number;
  currency: string;
  categoryId?: string | undefined;
  barcode?: string | undefined;
  productType?: string;
  taxRateIds: string[];
}

export const listProducts = (): Promise<ProductDto[]> =>
  invoke<ProductDto[]>('list_products');

/**
 * Fetch products scoped to the store resolved from a session token.
 *
 * ADR #4 / ADR #7 canonical pattern: The backend resolves the opaque
 * `sessionToken` to a `SessionContext` (containing `store_id`), opens
 * the store-scoped database, and returns only that store's products.
 *
 * Prefer this over the unscoped `listProducts()` in multi-store
 * deployments.
 */
export const listProductsScoped = (sessionToken: string): Promise<ProductDto[]> =>
  invoke<ProductDto[]>('list_products_scoped', { sessionToken });

export const createProduct = (args: CreateProductArgs): Promise<{ sku: string }> =>
  invoke('create_product', { args });

export const updateProduct = (args: UpdateProductArgs): Promise<{ sku: string }> =>
  invoke('update_product', { args });

export const deleteProduct = (args: { userId: string; sku: string }): Promise<void> =>
  invoke('delete_product', { args });

// ── Barcode / SKU Lookup ───────────────────────────────────────────

export const lookupByBarcode = (barcode: string): Promise<ProductDto | null> =>
  invoke<ProductDto | null>('lookup_by_barcode', { barcode });

/** ADR #7: Scoped barcode lookup using session token. */
export const lookupByBarcodeScoped = (sessionToken: string, barcode: string): Promise<ProductDto | null> =>
  invoke<ProductDto | null>('lookup_by_barcode_scoped', { sessionToken, barcode });

export const lookupProductBySku = (sku: string): Promise<ProductDto | null> =>
  invoke<ProductDto | null>('lookup_product_by_sku', { sku });

/** ADR #7: Scoped SKU lookup using session token. */
export const lookupProductBySkuScoped = (sessionToken: string, sku: string): Promise<ProductDto | null> =>
  invoke<ProductDto | null>('lookup_product_by_sku_scoped', { sessionToken, sku });

// ── Inventory Adjustment ──────────────────────────────────────────

export interface AdjustStockArgs {
  sku: string;
  delta: number;
  reason: string;
}

export const adjustStock = (args: AdjustStockArgs): Promise<number> =>
  invoke<number>('adjust_stock', { args });

/**
 * Adjust stock scoped to the store resolved from a session token.
 *
 * ADR #7: Prefer this over `adjustStock()` in multi-store deployments.
 */
export const adjustStockScoped = (sessionToken: string, args: AdjustStockArgs): Promise<number> =>
  invoke<number>('adjust_stock_scoped', { sessionToken, args });

// ── Product Variants ──────────────────────────────────────────────

export interface ProductVariantDto {
  id: string;
  parent_sku: string;
  name: string;
  sku: string;
  price: { minor_units: number; currency: string } | null;
  barcode: string | null;
  sort_order: number;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateProductVariantArgs {
  parentSku: string;
  name: string;
  sku: string;
  priceMinor?: number | null;
  currency?: string | null;
  barcode?: string | null;
  sortOrder?: number;
  isActive?: boolean;
}

export interface UpdateProductVariantArgs {
  sku: string;
  name?: string;
  priceMinor?: number | null;
  currency?: string | null;
  barcode?: string | null;
  sortOrder?: number;
  isActive?: boolean;
}

export const listProductVariants = (parentSku: string): Promise<ProductVariantDto[]> =>
  invoke<ProductVariantDto[]>('list_product_variants', { parentSku });

export const getProductVariant = (sku: string): Promise<ProductVariantDto | null> =>
  invoke<ProductVariantDto | null>('get_product_variant', { sku });

export const createProductVariant = (args: CreateProductVariantArgs): Promise<{ sku: string }> =>
  invoke<{ sku: string }>('create_product_variant', { args });

export const updateProductVariant = (args: UpdateProductVariantArgs): Promise<{ sku: string }> =>
  invoke<{ sku: string }>('update_product_variant', { args });

export const deleteProductVariant = (sku: string): Promise<void> =>
  invoke('delete_product_variant', { sku });

// ── Categories ────────────────────────────────────────────────────

export interface CategoryDto {
  id: string;
  name: string;
  colour: string;
  /** Icon identifier, e.g. "dots-1". Empty string = no icon. */
  icon: string;
}

export interface CreateCategoryArgs {
  id: string;
  name: string;
  colour: string;
  /** Icon identifier, e.g. "dots-1". */
  icon: string;
}

export interface UpdateCategoryArgs {
  id: string;
  name: string;
  colour: string;
  /** Icon identifier, e.g. "dots-2". */
  icon: string;
}

export const listCategories = (): Promise<CategoryDto[]> =>
  invoke<CategoryDto[]>('list_categories');

export const createCategory = (args: CreateCategoryArgs): Promise<{ id: string }> =>
  invoke('create_category', { args });

export const updateCategory = (args: UpdateCategoryArgs): Promise<{ id: string }> =>
  invoke('update_category', { args });

export const deleteCategory = (id: string): Promise<void> =>
  invoke('delete_category', { args: { id } });
