//! Sale endpoints — requires JWT authentication.

import { HttpClient } from './client';
import type {
  CreateSaleRequest,
  UpdateSaleStatusRequest,
} from './types';

export class SalesClient {
  constructor(private readonly http: HttpClient) {}

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
