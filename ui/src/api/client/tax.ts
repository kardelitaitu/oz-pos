//! Tax rate endpoints — requires JWT authentication.

import type { HttpClient } from './client';
import type { CreateTaxRateRequest } from './types';

export class TaxClient {
  constructor(private readonly http: HttpClient) {}

  /** `POST /api/v1/tax-rates` — create a new tax rate. */
  async create(req: CreateTaxRateRequest): Promise<void> {
    return this.http.request<void>('POST', '/api/v1/tax-rates', req);
  }
}
