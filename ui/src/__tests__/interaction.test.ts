import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// The interaction module has a module-level audioCache Map.
// We need to reset the module between tests so each test starts with a clean cache.
// Use vi.resetModules() + dynamic import.

let triggerInteraction: (name: string) => void;

async function importFresh() {
  const mod = await import('@/utils/interaction');
  triggerInteraction = mod.triggerInteraction;
  return mod;
}

describe('triggerInteraction', () => {
  beforeEach(async () => {
    vi.resetModules();
    // Mock Audio before import so the module-level audioCache starts fresh
    vi.spyOn(globalThis, 'Audio').mockImplementation(
      () =>
        ({
          volume: 0,
          currentTime: 0,
          play: vi.fn().mockResolvedValue(undefined),
        }) as unknown as HTMLAudioElement,
    );
    await importFresh();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('does nothing for an invalid interaction name', () => {
    expect(() => triggerInteraction('not-a-real-interaction')).not.toThrow();
  });

  it('creates an Audio element for add-to-cart interaction', () => {
    const audioSpy = vi.spyOn(globalThis, 'Audio');
    triggerInteraction('add-to-cart');
    expect(audioSpy).toHaveBeenCalledTimes(1);
    expect(audioSpy).toHaveBeenCalledWith(expect.stringContaining('click.mp3'));
  });

  it('caches audio element — creates Audio only once for repeated calls', () => {
    const audioSpy = vi.spyOn(globalThis, 'Audio');
    triggerInteraction('pay');
    triggerInteraction('pay');
    expect(audioSpy).toHaveBeenCalledTimes(1);
  });

  it('reuses cached audio across different interaction names (same sound file)', () => {
    const audioSpy = vi.spyOn(globalThis, 'Audio');
    triggerInteraction('add-to-cart');
    triggerInteraction('qty-change');
    triggerInteraction('remove-item');
    // All three use 'click.mp3' — only one Audio instance created
    expect(audioSpy).toHaveBeenCalledTimes(1);
  });

  it('handles audio play rejection gracefully', async () => {
    const mockPlay = vi.fn().mockRejectedValue(new Error('NotAllowedError'));
    vi.spyOn(globalThis, 'Audio').mockImplementation(
      () =>
        ({
          volume: 0,
          currentTime: 0,
          play: mockPlay,
        }) as unknown as HTMLAudioElement,
    );

    // Re-import with the new Audio mock
    await importFresh();

    expect(() => triggerInteraction('pay')).not.toThrow();

    // Wait for the rejected promise to settle
    await new Promise((r) => setTimeout(r, 10));
  });

  it('does not vibrate for standard interactions (vibrate: false in config)', () => {
    const vibrateSpy = vi.fn();
    navigator.vibrate = vibrateSpy;
    triggerInteraction('pay');
    expect(vibrateSpy).not.toHaveBeenCalled();
  });
});
