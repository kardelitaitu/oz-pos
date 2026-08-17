import { describe, expect, it } from 'vitest';
import { licenseApiUrl } from '../runtime-config';

/**
 * SSR/client parity for the backend URL (the "auth API is not configured"
 * flash bug class). The Astro server render must never see a value the
 * client then overrides — and the runtime config served by the Worker must
 * win over the build-time env on the client.
 */
const env = import.meta.env as Record<string, unknown>;

describe('licenseApiUrl (SSR/client parity)', () => {
  it('returns undefined without a window and without a build-time env (bare SSR)', () => {
    delete (globalThis as { window?: unknown }).window;
    delete env.PUBLIC_LICENSE_API_URL;
    expect(licenseApiUrl()).toBeUndefined();
  });

  it('prefers the Worker runtime config over the build-time value', () => {
    (globalThis as { window?: unknown }).window = {
      __OZ_CONFIG__: { licenseApiUrl: 'https://runtime.example' },
    };
    env.PUBLIC_LICENSE_API_URL = 'https://build.example';
    expect(licenseApiUrl()).toBe('https://runtime.example');
    delete (globalThis as { window?: unknown }).window;
  });

  it('falls back to the build-time env when the runtime config is absent', () => {
    delete (globalThis as { window?: unknown }).window;
    env.PUBLIC_LICENSE_API_URL = 'https://build.example';
    expect(licenseApiUrl()).toBe('https://build.example');
    delete env.PUBLIC_LICENSE_API_URL;
  });

  it('ignores a null runtime value (Worker var unset) and falls through', () => {
    (globalThis as { window?: unknown }).window = {
      __OZ_CONFIG__: { licenseApiUrl: null },
    };
    env.PUBLIC_LICENSE_API_URL = 'https://build.example';
    expect(licenseApiUrl()).toBe('https://build.example');
    delete (globalThis as { window?: unknown }).window;
    delete env.PUBLIC_LICENSE_API_URL;
  });
});
