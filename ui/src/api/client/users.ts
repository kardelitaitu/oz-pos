//! User account endpoints — requires JWT authentication.

import type { HttpClient } from './client';
import type { CreateUserRequest } from './types';

/** User record returned by the API. */
export interface User {
  id: string;
  username: string;
  role: string;
  active: boolean;
  created_at: string;
}

export class UsersClient {
  constructor(private readonly http: HttpClient) {}

  /** `GET /api/v1/users` — list all users. */
  async list(): Promise<User[]> {
    return this.http.request<User[]>('GET', '/api/v1/users');
  }

  /** `POST /api/v1/users` — create a new user account. */
  async create(req: CreateUserRequest): Promise<void> {
    return this.http.request<void>('POST', '/api/v1/users', req);
  }

  /** `GET /api/v1/users/{id}` — get user by ID. */
  async get(id: string): Promise<User | null> {
    return this.http.request<User | null>(
      'GET',
      `/api/v1/users/${encodeURIComponent(id)}`,
    );
  }

  /** `PUT /api/v1/users/{id}` — update a user. */
  async update(id: string, req: Partial<CreateUserRequest>): Promise<void> {
    return this.http.request<void>(
      'PUT',
      `/api/v1/users/${encodeURIComponent(id)}`,
      req,
    );
  }

  /** `DELETE /api/v1/users/{id}` — delete a user. */
  async delete(id: string): Promise<void> {
    return this.http.request<void>(
      'DELETE',
      `/api/v1/users/${encodeURIComponent(id)}`,
    );
  }
}
