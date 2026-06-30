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
}

export interface CreateProductArgs {
  sku: string;
  name: string;
  priceMinor: number;
  currency: string;
  categoryId?: string | undefined;
  barcode?: string | undefined;
  initialStock: number;
  taxRateIds: string[];
}

export interface UpdateProductArgs {
  sku: string;
  name: string;
  priceMinor: number;
  currency: string;
  categoryId?: string | undefined;
  barcode?: string | undefined;
  taxRateIds: string[];
}

export const listProducts = (): Promise<ProductDto[]> =>
  invoke<ProductDto[]>('list_products');

export const createProduct = (args: CreateProductArgs): Promise<{ sku: string }> =>
  invoke('create_product', { args });

export const updateProduct = (args: UpdateProductArgs): Promise<{ sku: string }> =>
  invoke('update_product', { args });

export const deleteProduct = (sku: string): Promise<void> =>
  invoke('delete_product', { args: { sku } });

// ── Barcode / SKU Lookup ───────────────────────────────────────────

export const lookupByBarcode = (barcode: string): Promise<ProductDto | null> =>
  invoke<ProductDto | null>('lookup_by_barcode', { barcode });

export const lookupProductBySku = (sku: string): Promise<ProductDto | null> =>
  invoke<ProductDto | null>('lookup_product_by_sku', { sku });

// ── Inventory Adjustment ──────────────────────────────────────────

export interface AdjustStockArgs {
  sku: string;
  delta: number;
  reason: string;
}

export const adjustStock = (args: AdjustStockArgs): Promise<number> =>
  invoke<number>('adjust_stock', { args });

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
}

export interface CreateCategoryArgs {
  id: string;
  name: string;
  colour: string;
}

export const listCategories = (): Promise<CategoryDto[]> =>
  invoke<CategoryDto[]>('list_categories');

export const createCategory = (args: CreateCategoryArgs): Promise<{ id: string }> =>
  invoke('create_category', { args });

export const deleteCategory = (id: string): Promise<void> =>
  invoke('delete_category', { args: { id } });
