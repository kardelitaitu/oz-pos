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
