import { invoke } from '@tauri-apps/api/core';

// ── Supplier types ──────────────────────────────────────────────────

/** A supplier record. */
export interface SupplierDto {
  id: string;
  code: string;
  name: string;
  contact_person: string;
  phone: string;
  email: string;
  address: string;
  tax_id: string;
  payment_terms: string;
  notes: string;
  status: string;
  created_at: string;
  updated_at: string;
}

/** Arguments for creating a new supplier. */
export interface CreateSupplierArgs {
  code: string;
  name: string;
  contact_person?: string;
  phone?: string;
  email?: string;
  address?: string;
  tax_id?: string;
  payment_terms?: string;
  notes?: string;
}

/** Arguments for updating an existing supplier. */
export interface UpdateSupplierArgs {
  id: string;
  code: string;
  name: string;
  contact_person?: string;
  phone?: string;
  email?: string;
  address?: string;
  tax_id?: string;
  payment_terms?: string;
  notes?: string;
  status?: string;
}

// ── Purchase Order types ────────────────────────────────────────────

/** A single line input for a purchase order. */
export interface PoLineInput {
  sku: string;
  product_name: string;
  qty: number;
  unit_cost_minor: number;
}

/** A line item within a purchase order. */
export interface PurchaseOrderLineDto {
  id: string;
  po_id: string;
  sku: string;
  product_name: string;
  qty: number;
  unit_cost_minor: number;
  line_total_minor: number;
}

/** A full purchase order with its line items. */
export interface PurchaseOrderDto {
  id: string;
  po_number: string;
  supplier_id: string;
  status: string;
  order_date: string;
  expected_date: string;
  received_date: string | null;
  subtotal_minor: number;
  tax_minor: number;
  total_minor: number;
  notes: string;
  created_by: string | null;
  created_at: string;
  updated_at: string;
  lines: PurchaseOrderLineDto[];
  supplier_name: string | null;
}

/** Arguments for creating a new purchase order. */
export interface CreatePurchaseOrderArgs {
  po_number: string;
  supplier_id: string;
  expected_date?: string;
  notes?: string;
  lines: PoLineInput[];
}

/** Arguments for updating a purchase order's status. */
export interface UpdatePoStatusArgs {
  id: string;
  status: string;
}

// ── Supplier API ────────────────────────────────────────────────────

/** List all suppliers. */
export const listSuppliers = (): Promise<SupplierDto[]> =>
  invoke<SupplierDto[]>('list_suppliers');

/** Get a single supplier by its identifier. */
export const getSupplier = (id: string): Promise<SupplierDto | null> =>
  invoke<SupplierDto | null>('get_supplier', { id });

/** Create a new supplier. */
export const createSupplier = (args: CreateSupplierArgs): Promise<SupplierDto> =>
  invoke<SupplierDto>('create_supplier', { args });

/** Update an existing supplier. */
export const updateSupplier = (args: UpdateSupplierArgs): Promise<SupplierDto> =>
  invoke<SupplierDto>('update_supplier', { args });

// ── Purchase Order API ──────────────────────────────────────────────

/** List all purchase orders. */
export const listPurchaseOrders = (): Promise<PurchaseOrderDto[]> =>
  invoke<PurchaseOrderDto[]>('list_purchase_orders');

/** Get a single purchase order by its identifier. */
export const getPurchaseOrder = (id: string): Promise<PurchaseOrderDto | null> =>
  invoke<PurchaseOrderDto | null>('get_purchase_order', { id });

/** Create a new purchase order. */
export const createPurchaseOrder = (args: CreatePurchaseOrderArgs): Promise<PurchaseOrderDto> =>
  invoke<PurchaseOrderDto>('create_purchase_order', { args });

/** Update a purchase order's status (e.g. approved, received, cancelled). */
export const updatePoStatus = (args: UpdatePoStatusArgs): Promise<PurchaseOrderDto> =>
  invoke<PurchaseOrderDto>('update_po_status', { args });

/** Mark a purchase order as received and update stock quantities. */
export const receivePurchaseOrder = (id: string): Promise<PurchaseOrderDto> =>
  invoke<PurchaseOrderDto>('receive_purchase_order', { id });
