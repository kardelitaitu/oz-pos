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

/** Bounded page of customer search results (CUST-06). */
export interface CustomerSearchPage {
  items: CustomerDto[];
  total: number;
}

/**
 * Server-side customer search (CUST-06): the query runs in the store DB
 * (LIKE over name/email/phone) with an explicit page size and total count,
 * so the renderer never holds the full customer list.
 */
export const searchCustomersScoped = (
  sessionToken: string,
  query: string,
  limit?: number,
  offset?: number,
): Promise<CustomerSearchPage> =>
  loggedInvoke<CustomerSearchPage>('search_customers_scoped', {
    sessionToken,
    query,
    limit,
    offset,
  });

/** A single sale in the customer history view (CUST-05). */
export interface CustomerSaleSummary {
  id: string;
  total_minor: number;
  currency: string;
  status: string;
  line_count: number;
  created_at: string;
}

/** Loyalty summary in the customer history view (CUST-05). */
export interface CustomerLoyaltySummary {
  points: number;
  lifetime_points: number;
  tier_name: string | null;
}

/** Read-only customer history: profile, loyalty summary, recent sales. */
export interface CustomerHistory {
  customer: CustomerDto;
  loyalty: CustomerLoyaltySummary | null;
  sales: CustomerSaleSummary[];
  sales_total: number;
}

/**
 * Get a customer's read-only history (CUST-05): profile, loyalty summary,
 * and a bounded most-recent-first page of sales.
 */
export const getCustomerHistoryScoped = (
  sessionToken: string,
  customerId: string,
  limit?: number,
  offset?: number,
): Promise<CustomerHistory> =>
  loggedInvoke<CustomerHistory>('get_customer_history_scoped', {
    sessionToken,
    customerId,
    limit,
    offset,
  });

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
