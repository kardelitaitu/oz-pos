import { describe, it, expect, beforeEach, vi } from 'vitest';
import {
  hexToRgb,
  rgbToHex,
  lighten,
  darken,
  rgba,
  deriveAccentPalette,
  applyAccentPalette,
} from '@/utils/color';

// ── hexToRgb ───────────────────────────────────────────────────────────
describe('hexToRgb', () => {
  it('parses a 6-char hex string', () => {
    expect(hexToRgb('#ff0000')).toEqual({ r: 255, g: 0, b: 0 });
    expect(hexToRgb('#00ff00')).toEqual({ r: 0, g: 255, b: 0 });
    expect(hexToRgb('#0000ff')).toEqual({ r: 0, g: 0, b: 255 });
  });

  it('parses a 3-char hex string by expanding each channel', () => {
    expect(hexToRgb('#f00')).toEqual({ r: 255, g: 0, b: 0 });
    expect(hexToRgb('#abc')).toEqual({ r: 170, g: 187, b: 204 });
  });

  it('handles hex without the hash prefix', () => {
    expect(hexToRgb('ff0000')).toEqual({ r: 255, g: 0, b: 0 });
  });

  it('parses white and black correctly', () => {
    expect(hexToRgb('#ffffff')).toEqual({ r: 255, g: 255, b: 255 });
    expect(hexToRgb('#000000')).toEqual({ r: 0, g: 0, b: 0 });
  });
});

// ── rgbToHex ───────────────────────────────────────────────────────────
describe('rgbToHex', () => {
  it('converts rgb values to hex', () => {
    expect(rgbToHex(255, 0, 0)).toBe('#ff0000');
    expect(rgbToHex(0, 255, 0)).toBe('#00ff00');
    expect(rgbToHex(0, 0, 255)).toBe('#0000ff');
  });

  it('converts white and black', () => {
    expect(rgbToHex(255, 255, 255)).toBe('#ffffff');
    expect(rgbToHex(0, 0, 0)).toBe('#000000');
  });

  it('clamps values to 0-255 range', () => {
    expect(rgbToHex(300, -10, 128)).toBe('#ff0080');
    expect(rgbToHex(-5, 500, 128)).toBe('#00ff80');
  });

  it('rounds floating point values', () => {
    expect(rgbToHex(127.6, 63.2, 0)).toBe('#803f00');
  });
});

// ── lighten ────────────────────────────────────────────────────────────
describe('lighten', () => {
  it('returns the original colour when amount is 0', () => {
    expect(lighten('#ff0000', 0)).toBe('#ff0000');
  });

  it('returns white when amount is 1', () => {
    expect(lighten('#ff0000', 1)).toBe('#ffffff');
    expect(lighten('#00ff00', 1)).toBe('#ffffff');
    expect(lighten('#0000ff', 1)).toBe('#ffffff');
  });

  it('blends toward white by the given amount', () => {
    // red halfway to white = #ff8080 (r=255, g=128, b=128)
    const result = lighten('#ff0000', 0.5);
    expect(result).toBe('#ff8080');
  });
});

// ── darken ─────────────────────────────────────────────────────────────
describe('darken', () => {
  it('returns the original colour when amount is 0', () => {
    expect(darken('#ff0000', 0)).toBe('#ff0000');
  });

  it('returns black when amount is 1', () => {
    expect(darken('#ff0000', 1)).toBe('#000000');
    expect(darken('#ffffff', 1)).toBe('#000000');
  });

  it('blends toward black by the given amount', () => {
    // red halfway to black = #800000 (r=128, g=0, b=0)
    const result = darken('#ff0000', 0.5);
    expect(result).toBe('#800000');
  });
});

// ── rgba ───────────────────────────────────────────────────────────────
describe('rgba', () => {
  it('produces an rgba() string with the given alpha', () => {
    expect(rgba('#ff0000', 0.5)).toBe('rgba(255, 0, 0, 0.5)');
    expect(rgba('#00ff00', 0.1)).toBe('rgba(0, 255, 0, 0.1)');
  });

  it('handles alpha of 1 (fully opaque)', () => {
    expect(rgba('#0000ff', 1)).toBe('rgba(0, 0, 255, 1)');
  });

  it('handles alpha of 0 (fully transparent)', () => {
    expect(rgba('#ffffff', 0)).toBe('rgba(255, 255, 255, 0)');
  });
});

// ── deriveAccentPalette ────────────────────────────────────────────────
describe('deriveAccentPalette', () => {
  it('uses the default emerald colour when no base is given', () => {
    const palette = deriveAccentPalette();
    expect(palette.base).toBe('#10b981');
  });

  it('derives hover, active, subtle, dim, alpha, secondary, and fg from base', () => {
    const palette = deriveAccentPalette('#4f46e5');
    expect(palette.base).toBe('#4f46e5');
    // hover = darken(base, 0.1)
    expect(palette.hover).toBeTruthy();
    expect(palette.hover).not.toBe(palette.base);
    // active = darken(base, 0.2)
    expect(palette.active).toBeTruthy();
    // subtle = lighten(base, ...)
    expect(palette.subtle).toBeTruthy();
    // fg is either '#0a0a0a' or '#ffffff'
    expect(['#0a0a0a', '#ffffff']).toContain(palette.fg);
    // dim = rgba(base, 0.12)
    expect(palette.dim).toBeTruthy();
    expect(palette.dim).toContain('rgba');
    // alpha = rgba(base, 0.2)
    expect(palette.alpha).toBeTruthy();
    expect(palette.alpha).toContain('rgba');
    // secondary = lighten(base, 0.15)
    expect(palette.secondary).toBeTruthy();
  });

  it('uses white fg for dark bases (luminance <= 160)', () => {
    const palette = deriveAccentPalette('#000000');
    expect(palette.fg).toBe('#ffffff');
  });

  it('uses dark fg for bright bases (luminance > 160)', () => {
    const palette = deriveAccentPalette('#ffffff');
    expect(palette.fg).toBe('#0a0a0a');
  });
});

// ── applyAccentPalette ─────────────────────────────────────────────────
describe('applyAccentPalette', () => {
  beforeEach(() => {
    // Reset any prior style properties
    const root = document.documentElement;
    root.style.removeProperty('--color-accent');
    root.style.removeProperty('--color-accent-hover');
    root.style.removeProperty('--color-accent-active');
    root.style.removeProperty('--color-accent-subtle');
    root.style.removeProperty('--color-accent-fg');
    root.style.removeProperty('--color-accent-dim');
    root.style.removeProperty('--color-accent-alpha');
    root.style.removeProperty('--color-accent-secondary');
  });

  it('sets all 8 CSS custom properties on documentElement', () => {
    const palette = deriveAccentPalette('#10b981');
    applyAccentPalette(palette);

    const root = document.documentElement;
    expect(root.style.getPropertyValue('--color-accent')).toBe(palette.base);
    expect(root.style.getPropertyValue('--color-accent-hover')).toBe(palette.hover);
    expect(root.style.getPropertyValue('--color-accent-active')).toBe(palette.active);
    expect(root.style.getPropertyValue('--color-accent-subtle')).toBe(palette.subtle);
    expect(root.style.getPropertyValue('--color-accent-fg')).toBe(palette.fg);
    expect(root.style.getPropertyValue('--color-accent-dim')).toBe(palette.dim);
    expect(root.style.getPropertyValue('--color-accent-alpha')).toBe(palette.alpha);
    expect(root.style.getPropertyValue('--color-accent-secondary')).toBe(palette.secondary);
  });
});
