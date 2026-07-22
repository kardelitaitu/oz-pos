//! Product endpoints — requires JWT authentication.

import type { HttpClient } from './client';
import type {
  CreateProductRequest,
  PatchStockRequest,
  PatchStockResponse,
  ProductDetail,
} from './types';

export class ProductsClient {
  constructor(private readonly http: HttpClient) {}

  /** `GET /api/v1/products` — list all products. */
  async list(): Promise<ProductDetail[]> {
    return this.http.request<ProductDetail[]>('GET', '/api/v1/products');
  }

  /** `POST /api/v1/products` — create a new product. */
  async create(req: CreateProductRequest): Promise<ProductDetail> {
    return this.http.request<ProductDetail>(
      'POST',
      '/api/v1/products',
      req,
    );
  }

  /** `GET /api/v1/products/{sku}` — get product by SKU. */
  async get(sku: string): Promise<ProductDetail | null> {
    return this.http.request<ProductDetail | null>(
      'GET',
      `/api/v1/products/${encodeURIComponent(sku)}`,
    );
  }

  /** `PUT /api/v1/products/{sku}` — update an existing product. */
  async update(
    sku: string,
    req: Partial<CreateProductRequest>,
  ): Promise<ProductDetail> {
    return this.http.request<ProductDetail>(
      'PUT',
      `/api/v1/products/${encodeURIComponent(sku)}`,
      req,
    );
  }

  /** `DELETE /api/v1/products/{sku}` — delete a product. */
  async delete(sku: string): Promise<void> {
    return this.http.request<void>(
      'DELETE',
      `/api/v1/products/${encodeURIComponent(sku)}`,
    );
  }

  /** `PATCH /api/v1/products/{sku}/stock` — adjust stock quantity. */
  async adjustStock(
    sku: string,
    req: PatchStockRequest,
  ): Promise<PatchStockResponse> {
    return this.http.request<PatchStockResponse>(
      'PATCH',
      `/api/v1/products/${encodeURIComponent(sku)}/stock`,
      req,
    );
  }
}
