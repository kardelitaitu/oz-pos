//! Product endpoints — requires JWT authentication.

import { HttpClient } from './client';
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
