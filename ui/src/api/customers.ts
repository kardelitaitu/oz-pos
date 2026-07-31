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

/** Arguments for creating a customer in the session's store. */
export interface CreateCustomerScopedArgs {
  name: string;
  email?: string;
  phone?: string;
  notes?: string;
}

/** Arguments for updating a customer in the session's store. */
export interface UpdateCustomerScopedArgs {
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

/** Create a customer in the store resolved from a session token. ADR #7. */
export const createCustomerScoped = (
  sessionToken: string,
  args: CreateCustomerScopedArgs,
): Promise<CustomerDto> =>
  loggedInvoke<CustomerDto>('create_customer_scoped', { sessionToken, args });

/** Update a customer in the store resolved from a session token. ADR #7. */
export const updateCustomerScoped = (
  sessionToken: string,
  args: UpdateCustomerScopedArgs,
): Promise<CustomerDto> =>
  loggedInvoke<CustomerDto>('update_customer_scoped', { sessionToken, args });

/** Delete a customer from the store resolved from a session token. ADR #7. */
export const deleteCustomerScoped = (sessionToken: string, id: string): Promise<void> =>
  loggedInvoke('delete_customer_scoped', { sessionToken, id });
