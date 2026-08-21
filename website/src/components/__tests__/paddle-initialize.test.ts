// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';

/**
 * Paddle SDK lifecycle. Regression: openPaddleCheckout used to call
 * Paddle.Initialize on EVERY open, but the v2 SDK's eventCallback is
 * registered by the first Initialize and is one-shot per page — a second
 * subscribe click could re-initialize and break the completion signal.
 */
describe('openPaddleCheckout SDK lifecycle', () => {
  beforeEach(() => {
    vi.resetModules();
    const env = import.meta.env as Record<string, unknown>;
    env.PUBLIC_PADDLE_CLIENT_TOKEN = 'test_token_123';
    env.PUBLIC_PADDLE_ENVIRONMENT = 'sandbox';
  });

  it('initializes the SDK once across multiple opens', async () => {
    const initialize = vi.fn();
    const open = vi.fn();
    const envSet = vi.fn();
    (window as unknown as { Paddle: unknown }).Paddle = {
      Environment: { set: envSet },
      Initialize: initialize,
      Checkout: { open },
    };

    const { openPaddleCheckout } = await import('../paddle');
    await openPaddleCheckout('pri_01m05gdnqp30xze6db73qcracp', 'a@b.com');
    await openPaddleCheckout('pri_01m05gdpk4hmnm0k8e6vxm8cec', 'a@b.com');

    expect(initialize).toHaveBeenCalledTimes(1);
    expect(open).toHaveBeenCalledTimes(2);
    expect(envSet).toHaveBeenCalledTimes(2);
    expect(envSet).toHaveBeenCalledWith('sandbox');

    delete (window as unknown as { Paddle?: unknown }).Paddle;
  });

  it('throws when no client token is configured', async () => {
    const env = import.meta.env as Record<string, unknown>;
    delete env.PUBLIC_PADDLE_CLIENT_TOKEN;
    const { openPaddleCheckout } = await import('../paddle');
    await expect(
      openPaddleCheckout('pri_01m05gdnqp30xze6db73qcracp', 'a@b.com'),
    ).rejects.toThrow('paddle not configured');
  });
});
