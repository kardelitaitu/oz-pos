// ── Customers: CRUD ───────────────────────────────────────────────

import { invoke } from '@tauri-apps/api/core';

export interface CustomerDto {
  id: string;
  name: string;
  email: string | null;
  phone: string | null;
  notes: string;
  created_at: string;
  updated_at: string;
}

export interface CreateCustomerArgs {
  name: string;
  email?: string;
  phone?: string;
  notes?: string;
}

export interface UpdateCustomerArgs {
  id: string;
  name: string;
  email?: string;
  phone?: string;
  notes?: string;
}

export const listCustomers = (): Promise<CustomerDto[]> =>
  invoke<CustomerDto[]>('list_customers');

export const getCustomer = (id: string): Promise<CustomerDto | null> =>
  invoke<CustomerDto | null>('get_customer', { id });

export const createCustomer = (args: CreateCustomerArgs): Promise<CustomerDto> =>
  invoke<CustomerDto>('create_customer', { args });

export const updateCustomer = (args: UpdateCustomerArgs): Promise<CustomerDto> =>
  invoke<CustomerDto>('update_customer', { args });

export const deleteCustomer = (id: string): Promise<void> =>
  invoke('delete_customer', { id });
