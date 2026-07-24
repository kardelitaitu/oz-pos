// ── Products: CRUD, variants, categories, barcode, stock adjustment ──

import { loggedInvoke } from '@/utils/logged-invoke';

// ── Products ──────────────────────────────────────────────────────

/** A product as returned by the backend. */
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

/** Arguments for creating a new product. */
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

/** Arguments for updating an existing product. */
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

/** List all products. */
export const listProducts = (): Promise<ProductDto[]> =>
  loggedInvoke<ProductDto[]>('list_products');

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
  loggedInvoke<ProductDto[]>('list_products_scoped', { sessionToken });

/** Create a new product. */
export const createProduct = (args: CreateProductArgs): Promise<{ sku: string }> =>
  loggedInvoke('create_product', { args });

/** ADR #7: Scoped product creation — `userId` is read from session, not args. */
export interface CreateProductScopedArgs {
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

export const createProductScoped = (sessionToken: string, args: CreateProductScopedArgs): Promise<{ sku: string }> =>
  loggedInvoke<{ sku: string }>('create_product_scoped', { sessionToken, args });

/** Update an existing product. */
export const updateProduct = (args: UpdateProductArgs): Promise<{ sku: string }> =>
  loggedInvoke('update_product', { args });

/** ADR #7: Scoped product update — `userId` is read from session, not args. */
export interface UpdateProductScopedArgs {
  sku: string;
  name: string;
  priceMinor: number;
  currency: string;
  categoryId?: string | undefined;
  barcode?: string | undefined;
  productType?: string;
  taxRateIds: string[];
}

export const updateProductScoped = (sessionToken: string, args: UpdateProductScopedArgs): Promise<{ sku: string }> =>
  loggedInvoke<{ sku: string }>('update_product_scoped', { sessionToken, args });

/** Delete a product by SKU. */
export const deleteProduct = (args: { userId: string; sku: string }): Promise<void> =>
  loggedInvoke('delete_product', { args });

/** ADR #7: Scoped product deletion — `userId` is read from session, not args. */
export const deleteProductScoped = (sessionToken: string, sku: string): Promise<void> =>
  loggedInvoke('delete_product_scoped', { sessionToken, args: { sku } });

// ── Barcode / SKU Lookup ───────────────────────────────────────────

/** Look up a product by its barcode. */
export const lookupByBarcode = (barcode: string): Promise<ProductDto | null> =>
  loggedInvoke<ProductDto | null>('lookup_by_barcode', { barcode });

/** ADR #7: Scoped barcode lookup using session token. */
export const lookupByBarcodeScoped = (sessionToken: string, barcode: string): Promise<ProductDto | null> =>
  loggedInvoke<ProductDto | null>('lookup_by_barcode_scoped', { sessionToken, barcode });

/** Look up a product by its SKU. */
export const lookupProductBySku = (sku: string): Promise<ProductDto | null> =>
  loggedInvoke<ProductDto | null>('lookup_product_by_sku', { sku });

/** Check whether a product tracks serial numbers. */
export const getProductTrackSerial = (sku: string): Promise<boolean> =>
  loggedInvoke<boolean>('get_product_track_serial', { sku });

/** Check whether a product tracks serial numbers, store-scoped. ADR #7. */
export const getProductTrackSerialScoped = (sessionToken: string, sku: string): Promise<boolean> =>
  loggedInvoke<boolean>('get_product_track_serial_scoped', { sessionToken, sku });

/** ADR #7: Scoped SKU lookup using session token. */
export const lookupProductBySkuScoped = (sessionToken: string, sku: string): Promise<ProductDto | null> =>
  loggedInvoke<ProductDto | null>('lookup_product_by_sku_scoped', { sessionToken, sku });

// ── Inventory Adjustment ──────────────────────────────────────────

/** Arguments for adjusting a product's stock quantity. */
export interface AdjustStockArgs {
  sku: string;
  delta: number;
  reason: string;
}

/** Adjust a product's stock level by a delta value. Returns the new stock quantity. */
export const adjustStock = (args: AdjustStockArgs): Promise<number> =>
  loggedInvoke<number>('adjust_stock', { args });

/**
 * Adjust stock scoped to the store resolved from a session token.
 *
 * ADR #7: Prefer this over `adjustStock()` in multi-store deployments.
 */
export const adjustStockScoped = (sessionToken: string, args: AdjustStockArgs): Promise<number> =>
  loggedInvoke<number>('adjust_stock_scoped', { sessionToken, args });

// ── Product Variants ──────────────────────────────────────────────

/** A product variant linked to a parent product. */
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

/** Arguments for creating a new product variant. */
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

/** Arguments for updating an existing product variant. */
export interface UpdateProductVariantArgs {
  sku: string;
  name?: string;
  priceMinor?: number | null;
  currency?: string | null;
  barcode?: string | null;
  sortOrder?: number;
  isActive?: boolean;
}

/** List all variants for a given parent product SKU. */
export const listProductVariants = (parentSku: string): Promise<ProductVariantDto[]> =>
  loggedInvoke<ProductVariantDto[]>('list_product_variants', { parentSku });

/** Get a single product variant by its SKU. */
export const getProductVariant = (sku: string): Promise<ProductVariantDto | null> =>
  loggedInvoke<ProductVariantDto | null>('get_product_variant', { sku });

/** Create a new product variant. */
export const createProductVariant = (args: CreateProductVariantArgs): Promise<{ sku: string }> =>
  loggedInvoke<{ sku: string }>('create_product_variant', { args });

/** Update an existing product variant. */
export const updateProductVariant = (args: UpdateProductVariantArgs): Promise<{ sku: string }> =>
  loggedInvoke<{ sku: string }>('update_product_variant', { args });

/** Delete a product variant by SKU. */
export const deleteProductVariant = (sku: string): Promise<void> =>
  loggedInvoke('delete_product_variant', { sku });

// ── Categories ────────────────────────────────────────────────────

export interface CategoryDto {
  id: string;
  name: string;
  colour: string;
  /** Icon identifier, e.g. "dots-1". Empty string = no icon. */
  icon: string;
}

/** Arguments for creating a new product category. */
export interface CreateCategoryArgs {
  id: string;
  name: string;
  colour: string;
  /** Icon identifier, e.g. "dots-1". */
  icon: string;
}

/** Arguments for updating an existing product category. */
export interface UpdateCategoryArgs {
  id: string;
  name: string;
  colour: string;
  /** Icon identifier, e.g. "dots-2". */
  icon: string;
}

/** List all product categories. */
export const listCategories = (): Promise<CategoryDto[]> =>
  loggedInvoke<CategoryDto[]>('list_categories');

/** List all product categories for the store resolved from a session token. ADR #7. */
export const listCategoriesScoped = (sessionToken: string): Promise<CategoryDto[]> =>
  loggedInvoke<CategoryDto[]>('list_categories_scoped', { sessionToken });

/** Create a new product category. */
export const createCategory = (args: CreateCategoryArgs): Promise<{ id: string }> =>
  loggedInvoke('create_category', { args });

/** Update an existing product category. */
export const updateCategory = (args: UpdateCategoryArgs): Promise<{ id: string }> =>
  loggedInvoke('update_category', { args });

/** Delete a product category by its identifier. */
export const deleteCategory = (id: string): Promise<void> =>
  loggedInvoke('delete_category', { args: { id } });
