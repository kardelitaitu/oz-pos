// ── Customers: CRUD ───────────────────────────────────────────────

import { loggedInvoke } from '@/utils/logged-invoke';

/** A customer record. */
export interface CustomerDto {
  id: string;
  name: string;
  email: string | null;
  phone: string | null;
  notes: string;
  created_at: string;
  updated_at: string;
}

/** Arguments for creating a new customer. */
export interface CreateCustomerArgs {
  userId: string;
  name: string;
  email?: string;
  phone?: string;
  notes?: string;
}

/** Arguments for updating an existing customer. */
export interface UpdateCustomerArgs {
  userId: string;
  id: string;
  name: string;
  email?: string;
  phone?: string;
  notes?: string;
}

/** List all customers. */
export const listCustomers = (): Promise<CustomerDto[]> =>
  loggedInvoke<CustomerDto[]>('list_customers');

/** List all customers for the store resolved from a session token. ADR #7. */
export const listCustomersScoped = (sessionToken: string): Promise<CustomerDto[]> =>
  loggedInvoke<CustomerDto[]>('list_customers_scoped', { sessionToken });

/** Get a single customer by their identifier. */
export const getCustomer = (id: string): Promise<CustomerDto | null> =>
  loggedInvoke<CustomerDto | null>('get_customer', { id });

/** Create a new customer. */
export const createCustomer = (args: CreateCustomerArgs): Promise<CustomerDto> =>
  loggedInvoke<CustomerDto>('create_customer', { args });

/** Update an existing customer. */
export const updateCustomer = (args: UpdateCustomerArgs): Promise<CustomerDto> =>
  loggedInvoke<CustomerDto>('update_customer', { args });

/** Delete a customer by their identifier. */
export const deleteCustomer = (args: { userId: string; id: string }): Promise<void> =>
  loggedInvoke('delete_customer', { args });
