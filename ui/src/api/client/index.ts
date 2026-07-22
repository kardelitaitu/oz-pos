//! Barrel export for the OZ-POS API client SDK.
//!
//! ```ts
//! import { OZPosClient } from '@/api/client';
//!
//! const client = new OZPosClient({ baseUrl: 'http://localhost:3099' });
//! client.setToken('eyJ...');
//! const health = await client.health.check();
//! const products = await client.products.list();
//! ```

export { OZPosClient } from './oz-pos-client';
export { ApiError, HttpClient, type ClientConfig, type HttpMethod } from './client';
export type * from './types';
export type { TaxRate } from './tax';
export type { User } from './users';
export type { SaleRecord } from './sales';
