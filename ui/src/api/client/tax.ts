//! Tax rate endpoints — requires JWT authentication.

import type { HttpClient } from './client';
import type { CreateTaxRateRequest } from './types';

/** Tax rate record returned by the API. */
export interface TaxRate {
  id: string;
  name: string;
  rate: number;
  category_id: string | null;
  product_id: string | null;
  active: boolean;
}

export class TaxClient {
  constructor(private readonly http: HttpClient) {}

  /** `GET /api/v1/tax-rates` — list all tax rates. */
  async list(): Promise<TaxRate[]> {
    return this.http.request<TaxRate[]>('GET', '/api/v1/tax-rates');
  }

  /** `POST /api/v1/tax-rates` — create a new tax rate. */
  async create(req: CreateTaxRateRequest): Promise<void> {
    return this.http.request<void>('POST', '/api/v1/tax-rates', req);
  }

  /** `GET /api/v1/tax-rates/{id}` — get tax rate by ID. */
  async get(id: string): Promise<TaxRate | null> {
    return this.http.request<TaxRate | null>(
      'GET',
      `/api/v1/tax-rates/${encodeURIComponent(id)}`,
    );
  }

  /** `PUT /api/v1/tax-rates/{id}` — update a tax rate. */
  async update(id: string, req: Partial<CreateTaxRateRequest>): Promise<void> {
    return this.http.request<void>(
      'PUT',
      `/api/v1/tax-rates/${encodeURIComponent(id)}`,
      req,
    );
  }

  /** `DELETE /api/v1/tax-rates/{id}` — delete a tax rate. */
  async delete(id: string): Promise<void> {
    return this.http.request<void>(
      'DELETE',
      `/api/v1/tax-rates/${encodeURIComponent(id)}`,
    );
  }
}
