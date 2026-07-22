//! Webhook endpoints — no authentication required.
//!
//! These are for sending test webhook events to the cloud server.
//! In production, Stripe/Square send webhooks directly to the server URL.

import { HttpClient } from './client';
import type { StripeWebhookEvent, SquareWebhookEvent } from './types';

export class WebhooksClient {
  constructor(private readonly http: HttpClient) {}

  /** `POST /api/webhooks/stripe` — send a Stripe webhook event. */
  async stripe(event: StripeWebhookEvent): Promise<void> {
    return this.http.request<void>('POST', '/api/webhooks/stripe', event);
  }

  /** `POST /api/webhooks/square` — send a Square webhook event. */
  async square(event: SquareWebhookEvent): Promise<void> {
    return this.http.request<void>('POST', '/api/webhooks/square', event);
  }
}
