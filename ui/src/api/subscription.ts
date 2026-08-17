// ── Subscription capabilities (C2.2 in-app upgrade triggers) ─────────

import { loggedInvoke } from '@/utils/logged-invoke';

/**
 * The tenant's tier quotas, feature flags, and current usage (C2.2).
 * Mirrors the Rust `SubscriptionCapabilitiesDto` in both clients —
 * a single local read that drives every in-app tier gate.
 */
export interface SubscriptionCapabilities {
  /** Tier key: `free` | `plus` | `pro` | `premium` | `enterprise`. */
  tier: string;
  // ── Quota limits (`null` = unlimited) ─────────────────────
  maxStores: number | null;
  maxPosInstances: number | null;
  maxWarehouses: number | null;
  maxStaffUsers: number | null;
  /** Free = 30 days; paid tiers = unlimited (`null`). */
  salesHistoryDays: number | null;
  // ── Feature flags ─────────────────────────────────────────
  supportsQris: boolean;
  supportsAnalytics: boolean;
  supportsLoyalty: boolean;
  supportsDailyDashboard: boolean;
  supportsCloudSync: boolean;
  offlineGraceDays: number;
  // ── Current usage (for approaching-limit banners) ──────────
  storeCount: number;
  staffCount: number;
  terminalCount: number;
}

/** Read the tenant's subscription capabilities (local, no network). */
export const getSubscriptionCapabilities = (): Promise<SubscriptionCapabilities> =>
  loggedInvoke<SubscriptionCapabilities>('get_subscription_capabilities');
