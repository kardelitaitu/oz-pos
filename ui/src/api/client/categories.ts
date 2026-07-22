//! Category endpoints — requires JWT authentication.

import { HttpClient } from './client';
import type { CategoryDto } from './types';

export class CategoriesClient {
  constructor(private readonly http: HttpClient) {}

  /** `GET /api/v1/categories` — list all categories. */
  async list(): Promise<CategoryDto[]> {
    return this.http.request<CategoryDto[]>('GET', '/api/v1/categories');
  }
}
