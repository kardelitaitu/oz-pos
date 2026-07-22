//! Category endpoints — requires JWT authentication.

import type { HttpClient } from './client';
import type { CategoryDto } from './types';

export class CategoriesClient {
  constructor(private readonly http: HttpClient) {}

  /** `GET /api/v1/categories` — list all categories. */
  async list(): Promise<CategoryDto[]> {
    return this.http.request<CategoryDto[]>('GET', '/api/v1/categories');
  }

  /** `POST /api/v1/categories` — create a new category. */
  async create(req: Omit<CategoryDto, 'id'>): Promise<CategoryDto> {
    return this.http.request<CategoryDto>('POST', '/api/v1/categories', req);
  }

  /** `GET /api/v1/categories/{id}` — get category by ID. */
  async get(id: string): Promise<CategoryDto | null> {
    return this.http.request<CategoryDto | null>(
      'GET',
      `/api/v1/categories/${encodeURIComponent(id)}`,
    );
  }

  /** `PUT /api/v1/categories/{id}` — update a category. */
  async update(id: string, req: Partial<CategoryDto>): Promise<CategoryDto> {
    return this.http.request<CategoryDto>(
      'PUT',
      `/api/v1/categories/${encodeURIComponent(id)}`,
      req,
    );
  }

  /** `DELETE /api/v1/categories/{id}` — delete a category. */
  async delete(id: string): Promise<void> {
    return this.http.request<void>(
      'DELETE',
      `/api/v1/categories/${encodeURIComponent(id)}`,
    );
  }
}
