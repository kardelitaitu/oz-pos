// ── UpdateBanner exit-animation tests ─────────────────────────────
//
// Pins the contract for the exit fade added in the sibling-surfaces
// polish. The banner should exit via a mirror `.update-banner--
// exiting` class (animation: update-banner-slide-out), not snap.
//
// Install path: Tauri's downloadAndInstall restarts the app, so the
// banner is unmounted via the `update.available` check transitioning
// to null on the next poll. Per the skill's "navigate to next state"
// rule, this should snap (no fade).
//
// Test consumer pattern: mock @tauri-apps/plugin-updater so we can
// deterministically control update.available + scripted install
// outcomes without touching the real Tauri runtime.

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { act } from 'react';
import { render, fireEvent } from '@testing-library/react';
import { FluentBundle, FluentResource } from '@fluent/bundle';
import { ReactLocalization, LocalizationProvider } from '@fluent/react';
import UpdateBanner from '@/components/UpdateBanner';

// ── Hoisted mocks ────────────────────────────────────────────────

const updaterMock = vi.hoisted(() => ({
  updateAvailable: true as boolean,
  installed: false as boolean,
  check: vi.fn(),
}));

vi.mock('@tauri-apps/plugin-updater', () => ({
  check: () =>
    updaterMock.updateAvailable && !updaterMock.installed
      ? Promise.resolve({
          version: '0.1.0',
          body: 'Bug fixes + new features',
          downloadAndInstall: vi.fn(async () => {
            updaterMock.installed = true;
            // Mimic Tauri restart: the new app process picks up the
            // installed state on its next check — our mock returns null
            // once installed=true, which is what would happen in
            // practice after a successful restart.
          }),
        })
      : Promise.resolve(null),
}));

const wrapper = ({ children }: { children: React.ReactNode }) => {
  const ftl = `
update-banner-title = Update available:
update-banner-new-version = new version
update-banner-install = Install
update-banner-installing = Installing…
update-banner-install-aria = Install update
update-banner-installing-aria = Installing update
update-banner-dismiss-aria = Dismiss update banner
`;
  const bundle = new FluentBundle('en');
  bundle.addResource(new FluentResource(ftl));
  const l10n = new ReactLocalization([bundle]);
  return <LocalizationProvider l10n={l10n}>{children}</LocalizationProvider>;
};

describe('UpdateBanner exit-animation polish', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    updaterMock.updateAvailable = true;
    updaterMock.installed = false;
    updaterMock.check.mockClear();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('does not render when no update is available', async () => {
    updaterMock.updateAvailable = false;
    render(<UpdateBanner />, { wrapper });
    // Flush the useEffect that calls `check()` so the banner can settle.
    await act(async () => { await vi.advanceTimersByTimeAsync(0); });
    expect(document.querySelector('.update-banner')).toBeNull();
  });

  it('renders the banner when an update is available', async () => {
    render(<UpdateBanner />, { wrapper });
    await act(async () => { await vi.advanceTimersByTimeAsync(0); });
    const banner = document.querySelector('.update-banner');
    expect(banner).toBeTruthy();
    expect(banner?.classList.contains('update-banner--exiting')).toBe(false);
  });

  it('applies the --exiting class when × is clicked, stays in DOM during fade', async () => {
    render(<UpdateBanner />, { wrapper });
    await act(async () => { await vi.advanceTimersByTimeAsync(0); });
    fireEvent.click(
      document.querySelector('.update-banner-btn--dismiss') as HTMLElement,
    );

    const banner = document.querySelector('.update-banner');
    expect(banner?.classList.contains('update-banner--exiting')).toBe(true);
    // Still in DOM during the 200ms fade.
    expect(document.querySelector('.update-banner')).toBeTruthy();
  });

  it('removes the banner from DOM after the 200 ms fade', async () => {
    render(<UpdateBanner />, { wrapper });
    await act(async () => { await vi.advanceTimersByTimeAsync(0); });
    fireEvent.click(
      document.querySelector('.update-banner-btn--dismiss') as HTMLElement,
    );

    act(() => { vi.advanceTimersByTime(199); });
    expect(document.querySelector('.update-banner')).toBeTruthy();

    act(() => { vi.advanceTimersByTime(1); });
    expect(document.querySelector('.update-banner')).toBeNull();
  });

  it('disables the dismiss button during the fade (no stale click race)', async () => {
    render(<UpdateBanner />, { wrapper });
    await act(async () => { await vi.advanceTimersByTimeAsync(0); });
    fireEvent.click(
      document.querySelector('.update-banner-btn--dismiss') as HTMLElement,
    );
    const btn = document.querySelector<HTMLButtonElement>(
      '.update-banner-btn--dismiss',
    );
    expect(btn?.disabled).toBe(true);
  });

  it('does not render an entry animation during the fade-out (idempotent dismiss)', async () => {
    render(<UpdateBanner />, { wrapper });
    await act(async () => { await vi.advanceTimersByTimeAsync(0); });
    fireEvent.click(
      document.querySelector('.update-banner-btn--dismiss') as HTMLElement,
    );
    expect(
      document.querySelector('.update-banner')?.classList.contains(
        'update-banner--exiting',
      ),
    ).toBe(true);

    // Click again mid-fade. Idempotent — still exiting, no double-timer.
    fireEvent.click(
      document.querySelector('.update-banner-btn--dismiss') as HTMLElement,
    );

    act(() => { vi.advanceTimersByTime(200); });
    expect(document.querySelector('.update-banner')).toBeNull();
  });

  it('install path does NOT trigger the fade (navigate-to-next-state)', async () => {
    render(<UpdateBanner />, { wrapper });
    // Flush the mount-time check() microtask so the banner renders.
    await act(async () => { await vi.advanceTimersByTimeAsync(0); });
    expect(document.querySelector('.update-banner')).toBeTruthy();

    fireEvent.click(
      document.querySelector('.update-banner-btn--primary') as HTMLElement,
    );

    // Flush the install Promise microtask. The handler sets
    // installing=true (no error path fires because the mock resolves
    // successfully), so the install button is now permanently disabled.
    // Tauri's real runtime would restart the app at this point.
    await act(async () => { await vi.advanceTimersByTimeAsync(0); });

    // Banner stays visible — no --exiting class. The skill's rule
    // for "navigate-to-next-state" is satisfied by the eventual app
    // restart, not by a fade-out within the current page lifetime.
    const banner = document.querySelector('.update-banner');
    expect(banner).toBeTruthy();
    expect(banner?.classList.contains('update-banner--exiting')).toBe(false);

    // Install button is now disabled (installing=true locks it).
    const installBtn = document.querySelector<HTMLButtonElement>(
      '.update-banner-btn--primary',
    );
    expect(installBtn?.disabled).toBe(true);

    // Mock received the install signal.
    expect(updaterMock.installed).toBe(true);
  });
});
