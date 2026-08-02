// ── Audit action/outcome catalog (AUD-08) ─────────────────────────
//
// Centralized mapping of emitted audit `action` strings to Fluent ids,
// plus outcome ids and the fallback ids used when an entry carries an
// unknown action or outcome. Keeping this in one module lets the screen,
// the parity tests, and future consumers (export, dashboards) share a
// single source of truth.

/** Map an emitted audit action string to its Fluent message id. */
export const ACTION_FLUENT_IDS: Record<string, string> = {
  'sale.void': 'audit-action-sale-void',
  'sale.complete': 'audit-action-sale-complete',
  'sale.completed': 'audit-action-sale-complete',
  'sale.create': 'audit-action-sale-create',
  'sale.refund': 'audit-action-sale-refund',
  'sale.refund.legacy': 'audit-action-sale-refund',
  'login': 'audit-action-login',
  'login.failed': 'audit-action-login-failed',
  'user.login': 'audit-action-login',
  'user.create': 'audit-action-user-create',
  'user.update': 'audit-action-user-update',
  'product.create': 'audit-action-product-create',
  'product.created': 'audit-action-product-create',
  'product.update': 'audit-action-product-update',
  'product.delete': 'audit-action-product-delete',
  'stock.adjust': 'audit-action-stock-adjust',
  'stock.adjusted': 'audit-action-stock-adjust',
  'setting.change': 'audit-action-setting-change',
  'setting.update': 'audit-action-setting-change',
  'settings.updated': 'audit-action-setting-change',
  'system.backup': 'audit-action-system-backup',
  'system.restore': 'audit-action-system-restore',
  'system.export': 'audit-action-system-export',
  'system.import': 'audit-action-system-import',
  'bulk.import': 'audit-action-bulk-import',
  'inventory.sync': 'audit-action-inventory-sync',
  'audit.review': 'audit-action-audit-review',
};

/** Safe localized label used when an action is not in the catalog. */
export const ACTION_FALLBACK_ID = 'audit-action-unknown';

/** Map an emitted audit `outcome` value to its Fluent message id. */
export const OUTCOME_FLUENT_IDS: Record<string, string> = {
  success: 'audit-log-outcome-success',
  failure: 'audit-log-outcome-failure',
};

/** Safe localized label used when an outcome is not success/failure. */
export const OUTCOME_FALLBACK_ID = 'audit-log-outcome-unknown';

/**
 * Actions considered critical/security for audit review. Used to render
 * the red critical bar and row emphasis.
 */
export const CRITICAL_ACTIONS = new Set([
  'login.failed', 'user.create', 'user.update',
  'setting.change', 'setting.update', 'settings.updated',
  'system.backup', 'system.restore',
  'system.export', 'system.import', 'bulk.import', 'product.delete',
]);
