# OZ-POS API Client SDK

TypeScript SDK for the OZ-POS cloud server REST API. Provides fully typed
access to all 20+ endpoints with Bearer token authentication.

## Quick Start

```ts
import { OZPosClient } from '@/api/client';

// Create a client pointing at your cloud server
const client = new OZPosClient({ baseUrl: 'http://localhost:3099' });

// (Optional) Set a Bearer token for authenticated endpoints
client.setToken('eyJhbGciOi...');

// Public endpoints — no token needed
const health = await client.health.check();
console.log(health.status); // "ok"

// Token management
const token = await client.auth.createToken({
  label: 'kitchen-display-1',
  expiry_hours: 24,
});

// Products
const allProducts = await client.products.list();
await client.products.create({
  sku: 'COFFEE-001',
  name: 'Espresso',
  price: { minor_units: 250, currency: 'USD' },
  initial_stock: 100,
});
const product = await client.products.get('COFFEE-001');
await client.products.adjustStock('COFFEE-001', { delta: -1 });

// Categories
const categories = await client.categories.list();

// Tax Rates
await client.tax.create({
  name: 'VAT 10%',
  rate_bps: 1000,
  is_default: true,
  is_inclusive: false,
});

// Users
await client.users.create({
  username: 'cashier1',
  pin_hash: 'hashed-pin',
  display_name: 'Cashier 1',
  role_id: 'role-cashier',
});

// Sales
await client.sales.create({
  lines: [{ sku: 'COFFEE-001', qty: 2, unit_price: { minor_units: 250, currency: 'USD' } }],
});
const sale = await client.sales.get('sale-id');
await client.sales.updateStatus('sale-id', { status: 'completed' });

// Sync
const syncStatus = await client.sync.status();
await client.sync.push([{ type: 'product', sku: 'COFFEE-001', name: 'Espresso' }]);
const pendingItems = await client.sync.pull({ since: null });

// Webhooks
await client.webhooks.stripe({ type: 'payment_intent.succeeded', data: {} });
await client.webhooks.square({ type: 'payment.updated', data: {} });
```

## API Reference

### Client Configuration

```ts
interface ClientConfig {
  baseUrl: string;              // Cloud server URL (e.g., http://localhost:3099)
  fetchFn?: typeof fetch;       // Custom fetch implementation (defaults to globalThis.fetch)
}
```

### Health

| Method | Endpoint | Auth | Returns |
|--------|----------|------|---------|
| `client.health.check()` | `GET /health` | No | `HealthResponse` |
| `client.health.checkApi()` | `GET /api/health` | No | `HealthResponse` |
| `client.health.metrics()` | `GET /metrics` | No | `string` (Prometheus text) |

### Auth

| Method | Endpoint | Auth | Returns |
|--------|----------|------|---------|
| `client.auth.createToken(req)` | `POST /api/v1/tokens` | No | `TokenResponse` |

### Products

| Method | Endpoint | Auth | Returns |
|--------|----------|------|---------|
| `client.products.list()` | `GET /api/v1/products` | Bearer | `ProductDetail[]` |
| `client.products.create(req)` | `POST /api/v1/products` | Bearer | `ProductDetail` |
| `client.products.get(sku)` | `GET /api/v1/products/{sku}` | Bearer | `ProductDetail \| null` |
| `client.products.adjustStock(sku, req)` | `PATCH /api/v1/products/{sku}/stock` | Bearer | `PatchStockResponse` |

### Categories

| Method | Endpoint | Auth | Returns |
|--------|----------|------|---------|
| `client.categories.list()` | `GET /api/v1/categories` | Bearer | `CategoryDto[]` |

### Tax Rates

| Method | Endpoint | Auth | Returns |
|--------|----------|------|---------|
| `client.tax.create(req)` | `POST /api/v1/tax-rates` | Bearer | `void` |

### Users

| Method | Endpoint | Auth | Returns |
|--------|----------|------|---------|
| `client.users.create(req)` | `POST /api/v1/users` | Bearer | `void` |

### Sales

| Method | Endpoint | Auth | Returns |
|--------|----------|------|---------|
| `client.sales.create(req)` | `POST /api/v1/sales` | Bearer | `void` |
| `client.sales.get(id)` | `GET /api/v1/sales/{id}` | Bearer | `Record \| null` |
| `client.sales.updateStatus(id, req)` | `PATCH /api/v1/sales/{id}` | Bearer | `void` |

### Sync

| Method | Endpoint | Auth | Returns |
|--------|----------|------|---------|
| `client.sync.status()` | `GET /api/sync/status` | Bearer | `SyncStatusResponse` |
| `client.sync.push(items)` | `POST /api/sync/push` | Bearer | `void` |
| `client.sync.pull(req?)` | `POST /api/sync/pull` | Bearer | `SyncQueueItem[]` |

### Webhooks

| Method | Endpoint | Auth | Returns |
|--------|----------|------|---------|
| `client.webhooks.stripe(event)` | `POST /api/webhooks/stripe` | No | `void` |
| `client.webhooks.square(event)` | `POST /api/webhooks/square` | No | `void` |

## Error Handling

All API errors are thrown as `ApiError` instances:

```ts
import { ApiError } from '@/api/client';

try {
  await client.products.create({ ... });
} catch (err) {
  if (err instanceof ApiError) {
    console.error(`HTTP ${err.status}: ${err.body}`);
  }
}
```

## Testing

The SDK is designed for easy testing via MSW or a custom `fetchFn`:

```ts
// Option 1: Custom fetch function
const client = new OZPosClient({
  baseUrl: 'http://test',
  fetchFn: async (url, init) => new Response(JSON.stringify({ status: 'ok' })),
});

// Option 2: MSW (recommended for integration tests)
import { http, HttpResponse } from 'msw';
// ... configure MSW handlers to intercept requests
```
