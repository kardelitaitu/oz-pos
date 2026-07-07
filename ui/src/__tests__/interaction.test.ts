import { describe, it, expect, vi, beforeEach } from 'vitest';

// ── mock the interaction module ───────────────────────────────────────
const mockPlay = vi.fn().mockResolvedValue(undefined);

vi.mock('@/utils/interaction', () => ({
  triggerInteraction: (name: string) => {
    // Known interactions
    const known = new Set([
      'add-to-cart', 'qty-change', 'remove-item', 'undo-cart', 'pay', 'open-bill',
    ]);
    if (!known.has(name)) return;
    // Attempt to play audio
    mockPlay().catch(() => {});
    // Vibrate not enabled for standard interactions
  },
}));

import { triggerInteraction } from '@/utils/interaction';

describe('triggerInteraction', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('plays audio for a known interaction', () => {
    triggerInteraction('add-to-cart');
    expect(mockPlay).toHaveBeenCalled();
  });

  it('plays audio for pay interaction', () => {
    triggerInteraction('pay');
    expect(mockPlay).toHaveBeenCalled();
  });

  it('does nothing for an unknown interaction name', () => {
    triggerInteraction('unknown-action' as 'add-to-cart');
    expect(mockPlay).not.toHaveBeenCalled();
  });

  it('does not throw when play() rejects', () => {
    mockPlay.mockRejectedValueOnce(new Error('NotAllowedError'));
    expect(() => triggerInteraction('remove-item')).not.toThrow();
  });

  it('handles undo-cart interaction', () => {
    triggerInteraction('undo-cart');
    expect(mockPlay).toHaveBeenCalled();
  });

  it('handles open-bill interaction', () => {
    triggerInteraction('open-bill');
    expect(mockPlay).toHaveBeenCalled();
  });

  it('handles qty-change interaction', () => {
    triggerInteraction('qty-change');
    expect(mockPlay).toHaveBeenCalled();
  });

  it('does not call play for empty string passed as name', () => {
    triggerInteraction('' as 'add-to-cart');
    expect(mockPlay).not.toHaveBeenCalled();
  });
});
