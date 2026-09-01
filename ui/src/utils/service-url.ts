// ── Service URLs (shared across auth / sync / status surfaces) ──────
// Unified deployment (auth + sync on one host, ADR #11) — the old
// standalone license service URL was folded into this single host.
export const AUTH_SERVICE_URL =
  (import.meta.env['VITE_AUTH_SERVICE_URL'] as string | undefined)
  ?? 'https://license.ozpos.my.id';
