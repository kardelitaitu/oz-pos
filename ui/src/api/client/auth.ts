//! Auth / Token endpoints.

import type { HttpClient } from './client';
import type { CreateTokenRequest, TokenResponse } from './types';

export class AuthClient {
  constructor(private readonly http: HttpClient) {}

  /** `POST /api/v1/tokens` — create a new JWT API token. */
  async createToken(req: CreateTokenRequest): Promise<TokenResponse> {
    return this.http.request<TokenResponse>('POST', '/api/v1/tokens', req);
  }
}
