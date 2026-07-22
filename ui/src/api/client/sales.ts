//! Sale endpoints — requires JWT authentication.

import type { HttpClient } from './client';
import type {
  CreateSaleRequest,
  UpdateSaleStatusRequest,
} from './types';

/** Sale record returned by the API. */
export interface SaleRecord {
  id: string;
  status: string;
  total_minor: number;
  currency: string;
  customer_id: string | null;
  created_at: string;
  updated_at: string;
}

export class SalesClient {
  constructor(private readonly http: HttpClient) {}

  /** `GET /api/v1/sales` — list all sales. */
  async list(): Promise<SaleRecord[]> {
    return this.http.request<SaleRecord[]>('GET', '/api/v1/sales');
  }

  /** `POST /api/v1/sales` — create a new sale (status: pending). */
  async create(req: CreateSaleRequest): Promise<void> {
    return this.http.request<void>('POST', '/api/v1/sales', req);
  }

  /** `GET /api/v1/sales/{id}` — get sale by ID. */
  async get(id: string): Promise<Record<string, unknown> | null> {
    return this.http.request<Record<string, unknown> | null>(
      'GET',
      `/api/v1/sales/${encodeURIComponent(id)}`,
    );
  }

  /** `PATCH /api/v1/sales/{id}` — update sale status (complete/void). */
  async updateStatus(
    id: string,
    req: UpdateSaleStatusRequest,
  ): Promise<void> {
    return this.http.request<void>(
      'PATCH',
      `/api/v1/sales/${encodeURIComponent(id)}`,
      req,
    );
  }
}
