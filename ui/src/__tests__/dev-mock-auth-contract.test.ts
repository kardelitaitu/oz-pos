import { afterEach, describe, expect, it, vi } from 'vitest';
// The vite config aliases @tauri-apps/api/core to the dev-mock in browser
// dev mode, so this test pins the mock's auth contract directly against
// the same module the app actually calls in dev.
//
// test-setup.ts globally mocks `@tauri-apps/api/event` (SettingsContext's
// dynamic import needs a stub in jsdom). This file imports the real
// dev-mock event module, so the global mock must be lifted here.
vi.unmock('@tauri-apps/api/event');
import { invoke, convertFileSrc, isTauri } from '@/dev-mock/tauri-api';
import { emit, listen } from '@/dev-mock/tauri-event';
import type { StaffLoginResult, BootstrapOwnerResult } from '@/api/staff';

// ── Real-webview delegation (Jul 2026 production regression) ───────────
// The vite alias used to be unconditional, so production builds bundled the
// mock and the packaged app silently ran on mock data. The mock must
// delegate to window.__TAURI_INTERNALS__ whenever a real Tauri webview is
// present, and only serve mock data in a plain browser (dev preview / E2E).

const fakeInternalsInvoke = vi.fn();

function installFakeInternals() {
  (window as unknown as { __TAURI_INTERNALS__: unknown }).__TAURI_INTERNALS__ = {
    invoke: fakeInternalsInvoke,
    transformCallback: (cb: (e: unknown) => void) => ({ id: 42, cb }),
    convertFileSrc: (p: string) => `asset://${p}`,
  };
}

function removeFakeInternals() {
  delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
}

afterEach(() => {
  removeFakeInternals();
  fakeInternalsInvoke.mockReset();
});

// audit/06 regression: the real backend mints a picker ticket at login and
// first-owner bootstrap. WorkspaceProvider bails (availableWorkspaces stays
// []) when pickerTicket is null, which renders "No workspaces available"
// for EVERY user in browser dev previews — even owner. The mock must keep
// parity with the backend contract or the picker can never load in dev.
describe('dev-mock auth contract (audit/06 picker ticket parity)', () => {
  it('staff_login returns a picker_ticket alongside the session', async () => {
    const result = (await invoke('staff_login', {
      args: { username: 'owner', pin: '1234' },
    })) as unknown as StaffLoginResult;

    expect(result.session.user_id).toBe('owner-1');
    expect(typeof result.picker_ticket).toBe('string');
    expect(result.picker_ticket.length).toBeGreaterThan(0);
    expect(result.picker_ticket).toMatch(/^mock-picker-/);
  });

  it('bootstrap_owner returns a picker_ticket alongside the session', async () => {
    const result = (await invoke('bootstrap_owner', {
      args: { username: 'new-owner', pin: '1234', display_name: 'New Owner' },
    })) as unknown as BootstrapOwnerResult;

    expect(result.session.role_name).toBe('owner');
    expect(typeof result.picker_ticket).toBe('string');
    expect(result.picker_ticket.length).toBeGreaterThan(0);
  });

  it('the picker listing is reachable once a ticket is returned', async () => {
    const login = (await invoke('staff_login', {
      args: { username: 'kasir', pin: '1234' },
    })) as unknown as StaffLoginResult;

    // The mock ignores the ticket (no real verification), but the listing
    // must return workspace instances so the picker renders in dev.
    const workspaces = (await invoke('list_workspaces', {
      ticket: login.picker_ticket,
      storeId: 'store-1',
    })) as unknown as unknown[];

    expect(Array.isArray(workspaces)).toBe(true);
    expect(workspaces.length).toBeGreaterThan(0);
  });
});

// The real backend persists the open shift (with opened_at) to the store
// `shifts` table, so a restart resumes the elapsed clock from the original
// opening time. The mock previously rebuilt the shift with `openedAt: new
// Date()` at module load, which reset the resto-POS "Current Order" shift
// duration to 0m on every reload. Pin the localStorage-backed contract so
// previews mirror the DB across page reloads.
describe('dev-mock active-shift persistence (restart parity)', () => {
  const KEY = 'oz-dev-mock:active-shift';

  afterEach(() => {
    localStorage.removeItem(KEY);
    vi.resetModules();
  });

  it('a previously opened shift survives a module reload with its openedAt', async () => {
    // Open a shift, then simulate a restart: re-import the module so the
    // in-memory copy is rebuilt from localStorage (like a fresh page load).
    const first = await import('@/dev-mock/tauri-api');
    const opened = (await first.invoke('open_shift_scoped', {
      sessionToken: 'session-1',
    })) as unknown as { openedAt: string };
    const openedAt = opened.openedAt;

    vi.resetModules();
    const second = await import('@/dev-mock/tauri-api');
    const reloaded = (await second.invoke('get_active_shift_scoped', {
      sessionToken: 'session-1',
    })) as unknown as { openedAt: string; status: string } | null;

    expect(reloaded).not.toBeNull();
    expect(reloaded?.status).toBe('open');
    // The clock must resume from the ORIGINAL opening time, not re-baseline
    // to the reload moment — otherwise the elapsed duration resets to 0m.
    expect(reloaded?.openedAt).toBe(openedAt);
  });

  it('closing the shift clears the persisted copy so a reload sees no active shift', async () => {
    const first = await import('@/dev-mock/tauri-api');
    await first.invoke('open_shift_scoped', { sessionToken: 'session-1' });
    await first.invoke('close_shift_scoped', {
      sessionToken: 'session-1',
      args: { id: 'shift-1', closingBalanceMinor: 0, notes: null },
    });

    vi.resetModules();
    const second = await import('@/dev-mock/tauri-api');
    const reloaded = (await second.invoke('get_active_shift_scoped', {
      sessionToken: 'session-1',
    })) as unknown as unknown;

    expect(reloaded).toBeNull();
  });
});

// The real backend persists active-cart lines and completed sales in the
// store DB, so a restart resumes exactly where the operator left off. The
// mock previously kept cartState / completedSales in module memory, so a
// reloaded preview dropped the in-progress cart and lost sales history.
// Pin the localStorage-backed contract so previews mirror the DB.
describe('dev-mock cart + sales persistence (restart parity)', () => {
  const CART_KEY = 'oz-dev-mock:cart';
  const SALES_KEY = 'oz-dev-mock:sales';

  afterEach(() => {
    localStorage.removeItem(CART_KEY);
    localStorage.removeItem(SALES_KEY);
    vi.resetModules();
  });

  it('an in-progress cart survives a module reload with its lines', async () => {
    const first = await import('@/dev-mock/tauri-api');
    await first.invoke('start_sale_scoped', { sessionToken: 'session-1' });
    await first.invoke('add_line_scoped', {
      sessionToken: 'session-1',
      args: { cartId: 'mock-cart-1', sku: 'LATTE', qty: 2 },
    });

    // Simulate a restart: re-import the module so the in-memory copy is
    // rebuilt from localStorage (like a fresh page load).
    vi.resetModules();
    const second = await import('@/dev-mock/tauri-api');
    const completed = (await second.invoke('complete_sale_scoped', {
      sessionToken: 'session-1',
    })) as unknown as { lineCount: number; total: { minor_units: number } };

    // The cart was rebuilt from storage, so the sale reflects the pre-reload lines.
    expect(completed.lineCount).toBe(1);
    expect(completed.total.minor_units).toBe(2 * 45000);
  });

  it('a completed sale survives a reload and its detail view resolves', async () => {
    const first = await import('@/dev-mock/tauri-api');
    await first.invoke('start_sale_scoped', { sessionToken: 'session-1' });
    await first.invoke('add_line_scoped', {
      sessionToken: 'session-1',
      args: { cartId: 'mock-cart-1', sku: 'LATTE', qty: 1 },
    });
    const sale = (await first.invoke('complete_sale_scoped', {
      sessionToken: 'session-1',
    })) as unknown as { saleId: string };

    vi.resetModules();
    const second = await import('@/dev-mock/tauri-api');
    const sales = (await second.invoke('list_sales_scoped', {
      sessionToken: 'session-1',
    })) as unknown as Array<{ id: string }>;
    const detail = (await second.invoke('get_sale_scoped', {
      id: sale.saleId,
    })) as unknown as { id: string; lines: unknown[] } | null;

    expect(sales.some(s => s.id === sale.saleId)).toBe(true);
    expect(detail?.id).toBe(sale.saleId);
    expect(detail?.lines.length).toBe(1);
  });

  it('starting a new sale clears the persisted cart so a reload starts fresh', async () => {
    const first = await import('@/dev-mock/tauri-api');
    await first.invoke('start_sale_scoped', { sessionToken: 'session-1' });
    await first.invoke('add_line_scoped', {
      sessionToken: 'session-1',
      args: { cartId: 'mock-cart-1', sku: 'LATTE', qty: 1 },
    });
    await first.invoke('start_sale_scoped', { sessionToken: 'session-1' });

    vi.resetModules();
    const second = await import('@/dev-mock/tauri-api');
    const completed = (await second.invoke('complete_sale_scoped', {
      sessionToken: 'session-1',
    })) as unknown as { lineCount: number };

    // The empty cart was persisted on start_sale, so the reloaded sale is empty.
    expect(completed.lineCount).toBe(0);
  });
});

describe('dev-mock delegates to a real Tauri webview (production regression)', () => {
  it('invoke passes through to window.__TAURI_INTERNALS__ when present', async () => {
    installFakeInternals();
    fakeInternalsInvoke.mockResolvedValue({ real: 'backend' });

    const result = await invoke('any_command', { a: 1 });

    // Delegated — the fake internals got the call, not the mock handlers.
    expect(fakeInternalsInvoke).toHaveBeenCalledWith('any_command', { a: 1 }, undefined);
    expect(result).toEqual({ real: 'backend' });
  });

  it('defaults missing args to {} like the real invoke signature', async () => {
    installFakeInternals();
    fakeInternalsInvoke.mockResolvedValue(undefined);

    await invoke('argument_less_command');

    expect(fakeInternalsInvoke).toHaveBeenCalledWith('argument_less_command', {}, undefined);
  });

  it('serves mock data in a plain browser (no internals)', async () => {
    const result = (await invoke('staff_login', {
      args: { username: 'owner', pin: '1234' },
    })) as unknown as StaffLoginResult;

    expect(fakeInternalsInvoke).not.toHaveBeenCalled();
    expect(result.session.user_id).toBe('owner-1');
  });

  it('convertFileSrc and isTauri reflect the webview state', () => {
    expect(isTauri()).toBe(false);
    expect(convertFileSrc('/tmp/x.png')).toBe('/tmp/x.png');

    installFakeInternals();
    expect(isTauri()).toBe(true);
    expect(convertFileSrc('/tmp/x.png')).toBe('asset:///tmp/x.png');
  });

  it('event emit/listen delegate to the real event plugin when present', async () => {
    installFakeInternals();
    fakeInternalsInvoke.mockResolvedValue(7);

    const unlisten = await listen('kds:orders-changed', () => {});
    await emit('kds:orders-changed', { id: 'x' });

    expect(fakeInternalsInvoke).toHaveBeenCalledWith('plugin:event|listen', expect.objectContaining({ event: 'kds:orders-changed' }));
    expect(fakeInternalsInvoke).toHaveBeenCalledWith('plugin:event|emit', {
      event: 'kds:orders-changed',
      payload: { id: 'x' },
    });

    await unlisten();
    expect(fakeInternalsInvoke).toHaveBeenCalledWith('plugin:event|unlisten', { event: 'kds:orders-changed', eventId: 7 });
  });
});

// The real backend persists KDS orders (kds_orders 032), per-item line
// statuses (kds_line_items 105), and the daily display counter
// (kds_daily_counters 032), so a restart resumes the kitchen queue exactly
// where the operator left off. The mock previously kept all three in module
// memory — a reloaded preview wiped the queue, reverted every status, and
// restarted ticket numbering at 104. Pin the localStorage-backed contract so
// previews mirror the DB across page reloads.
describe('dev-mock KDS persistence (restart parity)', () => {
  const KDS_KEY = 'oz-dev-mock:kds';

  afterEach(() => {
    localStorage.removeItem(KDS_KEY);
    vi.resetModules();
  });

  it('a pushed KDS order and its line items survive a module reload', async () => {
    const first = await import('@/dev-mock/tauri-api');
    await first.invoke('start_sale_scoped', { sessionToken: 'session-1' });
    await first.invoke('add_line_scoped', {
      sessionToken: 'session-1',
      args: { cartId: 'mock-cart-1', sku: 'ESPR', qty: 1 },
    });
    // complete_sale_scoped pushes a KDS order derived from the cart lines.
    // '1x Espresso Shot' is unambiguous — the seeds carry different summaries.
    await first.invoke('complete_sale_scoped', { sessionToken: 'session-1' });

    // Simulate a restart: re-import the module so the in-memory copy is
    // rebuilt from localStorage (like a fresh page load).
    vi.resetModules();
    const second = await import('@/dev-mock/tauri-api');
    const queue = (await second.invoke('get_kds_queue_scoped', {
      sessionToken: 'session-1',
    })) as unknown as Array<Record<string, unknown>>;

    // The pushed order must be in the reloaded queue, not just the 3 seeds.
    const pushed = queue.find(
      (o) => o['items_summary'] === '1x Espresso Shot' && o['status'] === 'pending',
    );
    expect(pushed).toBeDefined();
    // …and its course-grouped line items must survive so the ticket renders.
    const lines = (await second.invoke('get_kds_order_lines_scoped', {
      sessionToken: 'session-1',
      args: { orderId: pushed?.['id'] as string },
    })) as unknown as unknown[];
    expect(lines.length).toBe(1);
  });

  it('the display counter continues past the pre-reload ticket number', async () => {
    const first = await import('@/dev-mock/tauri-api');
    await first.invoke('start_sale_scoped', { sessionToken: 'session-1' });
    await first.invoke('add_line_scoped', {
      sessionToken: 'session-1',
      args: { cartId: 'mock-cart-1', sku: 'LATTE', qty: 1 },
    });
    await first.invoke('complete_sale_scoped', { sessionToken: 'session-1' }); // ticket 104

    vi.resetModules();
    const second = await import('@/dev-mock/tauri-api');
    await second.invoke('start_sale_scoped', { sessionToken: 'session-1' });
    await second.invoke('add_line_scoped', {
      sessionToken: 'session-1',
      args: { cartId: 'mock-cart-1', sku: 'ESPR', qty: 1 },
    });
    await second.invoke('complete_sale_scoped', { sessionToken: 'session-1' });

    const queue = (await second.invoke('get_kds_queue_scoped', {
      sessionToken: 'session-1',
    })) as unknown as Array<Record<string, unknown>>;
    const displayNumbers = queue.map((o) => Number(o['display_number']));
    // Ticket numbering must continue past 104 — not restart at the seed.
    expect(displayNumbers).toContain(105);
  });

  it('a line-item status update survives a module reload', async () => {
    const first = await import('@/dev-mock/tauri-api');
    // Flip a seeded order's line item to 'preparing' (per-item advance).
    const flipped = (await first.invoke('update_kds_line_item_status', {
      args: { itemId: 'kds-line-1-1', status: 'preparing' },
    })) as unknown as { item_status: string } | null;
    expect(flipped?.item_status).toBe('preparing');

    vi.resetModules();
    const second = await import('@/dev-mock/tauri-api');
    const lines = (await second.invoke('get_kds_order_lines', {
      args: { id: 'kds-order-1' },
    })) as unknown as Array<Record<string, unknown>>;
    expect(lines.find((l) => l['id'] === 'kds-line-1-1')?.['item_status']).toBe('preparing');
  });
});

// The real backend persists login attempts (login_attempts 074 + device
// 111) and closed-shift history (shifts 021), so a reload cannot defeat an
// active lockout and reconciliation history survives restarts. The mock
// kept both in module memory — a reloaded preview bypassed the lockout and
// lost every closed shift (only the seed regenerated). Pin the
// localStorage-backed contracts so previews mirror the DB.
describe('dev-mock lockout + shift-history persistence (restart parity)', () => {
  const LOCK_KEY = 'oz-dev-mock:login-attempts';
  const SHIFT_KEY = 'oz-dev-mock:shift-history';

  afterEach(() => {
    localStorage.removeItem(LOCK_KEY);
    localStorage.removeItem(SHIFT_KEY);
    vi.resetModules();
  });

  it('a lockout survives a module reload so a reloaded page cannot bypass it', async () => {
    const first = await import('@/dev-mock/tauri-api');
    // Four failed attempts for kasir → the fifth is blocked.
    for (let i = 0; i < 4; i++) {
      await expect(
        first.invoke('staff_login', { args: { username: 'kasir', pin: '0000' } }),
      ).rejects.toThrow(/Invalid credentials/);
    }

    // Simulate a restart: re-import the module so the in-memory copy is
    // rebuilt from localStorage (like a fresh page load).
    vi.resetModules();
    const second = await import('@/dev-mock/tauri-api');
    // Even with the CORRECT pin, the persisted attempt count still blocks.
    await expect(
      second.invoke('staff_login', { args: { username: 'kasir', pin: '1234' } }),
    ).rejects.toThrow(/Account locked/);
  });

  it('a successful login clears the persisted lockout', async () => {
    const first = await import('@/dev-mock/tauri-api');
    for (let i = 0; i < 3; i++) {
      await expect(
        first.invoke('staff_login', { args: { username: 'kasir', pin: '0000' } }),
      ).rejects.toThrow(/Invalid credentials/);
    }
    // Correct pin resets the counter (the persisted delete must survive).
    await first.invoke('staff_login', { args: { username: 'kasir', pin: '1234' } });

    vi.resetModules();
    const second = await import('@/dev-mock/tauri-api');
    // A wrong pin afterwards is a fresh first failure, not a lockout.
    await expect(
      second.invoke('staff_login', { args: { username: 'kasir', pin: '0000' } }),
    ).rejects.toThrow(/Invalid credentials/);
  });

  it('a closed shift survives a module reload in the shift history', async () => {
    const first = await import('@/dev-mock/tauri-api');
    await first.invoke('open_shift_scoped', { sessionToken: 'session-1' });
    const closed = (await first.invoke('close_shift_scoped', {
      sessionToken: 'session-1',
      args: { id: 'shift-1', closingBalanceMinor: 0, notes: null },
    })) as unknown as { id: string };

    vi.resetModules();
    const second = await import('@/dev-mock/tauri-api');
    const shifts = (await second.invoke('list_shifts_scoped', {
      sessionToken: 'session-1',
    })) as unknown as Array<Record<string, unknown>>;

    // The closed shift must be in the reloaded history, not just the seed.
    expect(shifts.some((s) => s['id'] === closed.id)).toBe(true);
  });

  it('a fresh browser seeds exactly the one pre-seeded closed shift', async () => {
    const first = await import('@/dev-mock/tauri-api');
    const shifts = (await first.invoke('list_shifts_scoped', {
      sessionToken: 'session-1',
    })) as unknown as Array<Record<string, unknown>>;
    expect(shifts.length).toBe(1);
    expect(shifts[0]!['id']).toBe('shift-seed-1');
  });
});
