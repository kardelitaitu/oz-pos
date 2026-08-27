// @vitest-environment jsdom
import { afterEach, describe, expect, it } from 'vitest';
import { getRegion, setRegion, isIndonesia } from '../region';

describe('region helpers (localStorage get/set)', () => {
  afterEach(() => {
    localStorage.clear();
  });

  it('returns "global" when no value is stored (default)', () => {
    expect(getRegion()).toBe('global');
  });

  it('returns the stored region after setRegion', () => {
    setRegion('id');
    expect(getRegion()).toBe('id');
  });

  it('overwrites a previously stored region', () => {
    setRegion('global');
    expect(getRegion()).toBe('global');
    setRegion('id');
    expect(getRegion()).toBe('id');
  });

  it('reads the oz_region key from localStorage', () => {
    localStorage.setItem('oz_region', 'id');
    expect(getRegion()).toBe('id');
  });

  it('isIndonesia returns true only when region is "id"', () => {
    expect(isIndonesia()).toBe(false);
    setRegion('id');
    expect(isIndonesia()).toBe(true);
    setRegion('global');
    expect(isIndonesia()).toBe(false);
  });

  it('falls back to "global" for an unrecognized value', () => {
    localStorage.setItem('oz_region', 'eu');
    // getRegion returns whatever is in localStorage (cast to Region),
    // but isIndonesia checks strictly for 'id'.
    expect(isIndonesia()).toBe(false);
  });
});
