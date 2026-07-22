//! Health endpoints — no authentication required.

import { HttpClient } from './client';
import type { HealthResponse } from './types';

export class HealthClient {
  constructor(private readonly http: HttpClient) {}

  /** `GET /health` — basic health check. */
  async check(): Promise<HealthResponse> {
    return this.http.request<HealthResponse>('GET', '/health');
  }

  /** `GET /api/health` — API alias for health check. */
  async checkApi(): Promise<HealthResponse> {
    return this.http.request<HealthResponse>('GET', '/api/health');
  }

  /** `GET /metrics` — Prometheus metrics (returns plain text, not JSON). */
  async metrics(): Promise<string> {
    return this.http.requestRaw('GET', '/metrics');
  }
}
