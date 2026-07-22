//! User account endpoints — requires JWT authentication.

import type { HttpClient } from './client';
import type { CreateUserRequest } from './types';

export class UsersClient {
  constructor(private readonly http: HttpClient) {}

  /** `POST /api/v1/users` — create a new user account. */
  async create(req: CreateUserRequest): Promise<void> {
    return this.http.request<void>('POST', '/api/v1/users', req);
  }
}
