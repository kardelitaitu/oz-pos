import { invoke } from '@tauri-apps/api/core';

// ── Supplier types ──────────────────────────────────────────────────

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

export interface PoLineInput {
  sku: string;
  product_name: string;
  qty: number;
  unit_cost_minor: number;
}

export interface PurchaseOrderLineDto {
  id: string;
  po_id: string;
  sku: string;
  product_name: string;
  qty: number;
  unit_cost_minor: number;
  line_total_minor: number;
}

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

export interface CreatePurchaseOrderArgs {
  po_number: string;
  supplier_id: string;
  expected_date?: string;
  notes?: string;
  lines: PoLineInput[];
}

export interface UpdatePoStatusArgs {
  id: string;
  status: string;
}

// ── Supplier API ────────────────────────────────────────────────────

export const listSuppliers = (): Promise<SupplierDto[]> =>
  invoke<SupplierDto[]>('list_suppliers');

export const getSupplier = (id: string): Promise<SupplierDto | null> =>
  invoke<SupplierDto | null>('get_supplier', { id });

export const createSupplier = (args: CreateSupplierArgs): Promise<SupplierDto> =>
  invoke<SupplierDto>('create_supplier', { args });

export const updateSupplier = (args: UpdateSupplierArgs): Promise<SupplierDto> =>
  invoke<SupplierDto>('update_supplier', { args });

// ── Purchase Order API ──────────────────────────────────────────────

export const listPurchaseOrders = (): Promise<PurchaseOrderDto[]> =>
  invoke<PurchaseOrderDto[]>('list_purchase_orders');

export const getPurchaseOrder = (id: string): Promise<PurchaseOrderDto | null> =>
  invoke<PurchaseOrderDto | null>('get_purchase_order', { id });

export const createPurchaseOrder = (args: CreatePurchaseOrderArgs): Promise<PurchaseOrderDto> =>
  invoke<PurchaseOrderDto>('create_purchase_order', { args });

export const updatePoStatus = (args: UpdatePoStatusArgs): Promise<PurchaseOrderDto> =>
  invoke<PurchaseOrderDto>('update_po_status', { args });

export const receivePurchaseOrder = (id: string): Promise<PurchaseOrderDto> =>
  invoke<PurchaseOrderDto>('receive_purchase_order', { id });
