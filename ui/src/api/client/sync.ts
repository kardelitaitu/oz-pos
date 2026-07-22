//! Sync endpoints — requires JWT authentication.
//!
//! These are the offline queue push/pull endpoints used by terminals
//! to sync data with the cloud server.

import { HttpClient } from './client';
import type { SyncPullRequest, SyncQueueItem, SyncStatusResponse } from './types';

export class SyncClient {
  constructor(private readonly http: HttpClient) {}

  /** `GET /api/sync/status` — get sync queue state. */
  async status(): Promise<SyncStatusResponse> {
    return this.http.request<SyncStatusResponse>('GET', '/api/sync/status');
  }

  /** `POST /api/sync/push` — push offline items to the server. */
  async push(items: SyncQueueItem[]): Promise<void> {
    return this.http.request<void>('POST', '/api/sync/push', items);
  }

  /** `POST /api/sync/pull` — pull pending items from the server. */
  async pull(req?: SyncPullRequest): Promise<SyncQueueItem[]> {
    const body = req ?? { since: null };
    return this.http.request<SyncQueueItem[]>('POST', '/api/sync/pull', body);
  }
}
