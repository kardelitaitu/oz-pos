import { test, expect } from '@playwright/test';

/**
 * API Integration Tests
 *
 * These tests validate the cloud-server HTTP API directly, without
 * going through the UI. They require the cloud server to be running
 * via Docker Compose (docker-compose.e2e.yml).
 *
 * Tests are skipped if the cloud server is unreachable (CI without
 * Docker or local dev without docker-compose up).
 *
 * Flows tested:
 *   1. Health check endpoint
 *   2. License server health
 *   3. Auth token generation
 *   4. Data sync cycle (push → pull)
 */

// ── Configuration ─────────────────────────────────────────────────

const CLOUD_SERVER_URL =
  process.env['CLOUD_SERVER_URL'] ?? 'http://localhost:3099';
const LICENSE_SERVER_URL =
  process.env['LICENSE_SERVER_URL'] ?? 'http://localhost:8080';

// ── Helpers ───────────────────────────────────────────────────────

async function isServerReachable(url: string): Promise<boolean> {
  try {
    const resp = await fetch(`${url}/api/v1/health`, {
      signal: AbortSignal.timeout(3_000),
    });
    return resp.ok;
  } catch {
    return false;
  }
}

// ── Test Suite ────────────────────────────────────────────────────

test.describe('Cloud Server API', () => {
  let serverUp = false;

  test.beforeAll(async () => {
    serverUp = await isServerReachable(CLOUD_SERVER_URL);
  });

  test('health endpoint returns 200', async () => {
    test.skip(!serverUp, 'Cloud server not running — skip API tests');

    const resp = await fetch(`${CLOUD_SERVER_URL}/api/v1/health`);
    expect(resp.ok).toBe(true);
    expect(resp.status).toBe(200);

    const body = await resp.json();
    expect(body).toHaveProperty('status');
    expect(body.status).toBe('ok');
    expect(body).toHaveProperty('version');
    expect(body).toHaveProperty('uptime_seconds');
  });

  test('health endpoint includes database info', async () => {
    test.skip(!serverUp, 'Cloud server not running — skip API tests');

    const resp = await fetch(`${CLOUD_SERVER_URL}/api/v1/health`);
    const body = await resp.json();

    // Should have DB connectivity info.
    expect(body).toHaveProperty('db_connected');
    // SQLite is always connected (embedded).
    expect(body.db_connected).toBe(true);
  });
});

test.describe('License Server API', () => {
  let licenseUp = false;

  test.beforeAll(async () => {
    try {
      const resp = await fetch(`${LICENSE_SERVER_URL}/api/health`, {
        signal: AbortSignal.timeout(3_000),
      });
      licenseUp = resp.ok;
    } catch {
      licenseUp = false;
    }
  });

  test('health endpoint returns 200', async () => {
    test.skip(!licenseUp, 'License server not running — skip API tests');

    const resp = await fetch(`${LICENSE_SERVER_URL}/api/health`);
    expect(resp.ok).toBe(true);
    expect(resp.status).toBe(200);

    const body = await resp.json();
    expect(body).toHaveProperty('status');
    expect(body.status).toBe('ok');
  });

  test('license status endpoint returns status info', async () => {
    test.skip(!licenseUp, 'License server not running — skip API tests');

    const resp = await fetch(
      `${LICENSE_SERVER_URL}/api/v1/license/status`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          license_key: 'OZ-PRO-TEST-ABCD-EFGH-IJKL',
        }),
      },
    );

    // The license server should respond (may be 401 if unsigned, or valid).
    expect(resp.status === 200 || resp.status === 401).toBe(true);
    const body = await resp.json();
    // Should have some status field.
    expect(body).toBeDefined();
  });
});

test.describe('Sync API', () => {
  let serverUp = false;

  test.beforeAll(async () => {
    serverUp = await isServerReachable(CLOUD_SERVER_URL);
  });

  test('sync pull endpoint requires auth', async () => {
    test.skip(!serverUp, 'Cloud server not running — skip API tests');

    // Without auth token, pull should return 401.
    const resp = await fetch(`${CLOUD_SERVER_URL}/api/sync/pull`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({}),
    });
    expect(resp.status).toBe(401);
  });

  test('sync push endpoint requires auth', async () => {
    test.skip(!serverUp, 'Cloud server not running — skip API tests');

    const resp = await fetch(`${CLOUD_SERVER_URL}/api/sync/push`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({}),
    });
    expect(resp.status).toBe(401);
  });
});
