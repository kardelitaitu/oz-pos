import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
  hexToRgb,
  rgbToHex,
  lighten,
  darken,
  rgba,
  deriveAccentPalette,
  applyAccentPalette,
} from '@/utils/color';

describe('hexToRgb', () => {
  it('parses a 6-digit hex string', () => {
    expect(hexToRgb('#ff0000')).toEqual({ r: 255, g: 0, b: 0 });
  });

  it('parses a 3-digit hex string', () => {
    expect(hexToRgb('#f00')).toEqual({ r: 255, g: 0, b: 0 });
  });

  it('parses white', () => {
    expect(hexToRgb('#ffffff')).toEqual({ r: 255, g: 255, b: 255 });
  });

  it('parses black', () => {
    expect(hexToRgb('#000000')).toEqual({ r: 0, g: 0, b: 0 });
  });

  it('parses a hex string without hash prefix', () => {
    expect(hexToRgb('10b981')).toEqual({ r: 16, g: 185, b: 129 });
  });
});

describe('rgbToHex', () => {
  it('converts rgb to hex', () => {
    expect(rgbToHex(255, 0, 0)).toBe('#ff0000');
  });

  it('pads single-digit hex values', () => {
    expect(rgbToHex(16, 185, 129)).toBe('#10b981');
  });

  it('clamps values to 0–255', () => {
    expect(rgbToHex(300, -10, 128)).toBe('#ff0080');
  });

  it('produces lowercase hex', () => {
    expect(rgbToHex(170, 187, 204)).toBe('#aabbcc');
  });
});

describe('lighten', () => {
  it('returns the same colour for amount 0', () => {
    expect(lighten('#ff0000', 0)).toBe('#ff0000');
  });

  it('returns white for amount 1', () => {
    expect(lighten('#ff0000', 1)).toBe('#ffffff');
    expect(lighten('#00ff00', 1)).toBe('#ffffff');
  });

  it('lightens a dark colour', () => {
    const result = lighten('#10b981', 0.5);
    // r: 16 + (255-16)*0.5 = 135.5 → round 136 → 88
    // g: 185 + (255-185)*0.5 = 220 → dc
    // b: 129 + (255-129)*0.5 = 192 → c0
    expect(result).toBe('#88dcc0');
  });
});

describe('darken', () => {
  it('returns the same colour for amount 0', () => {
    expect(darken('#ff0000', 0)).toBe('#ff0000');
  });

  it('returns black for amount 1', () => {
    expect(darken('#ff0000', 1)).toBe('#000000');
    expect(darken('#ffffff', 1)).toBe('#000000');
  });

  it('darkens a light colour', () => {
    const result = darken('#ffffff', 0.5);
    // 255 * 0.5 = 127.5 → round 128 → 80
    expect(result).toBe('#808080');
  });
});

describe('rgba', () => {
  it('formats rgba with given alpha', () => {
    expect(rgba('#ff0000', 0.5)).toBe('rgba(255, 0, 0, 0.5)');
  });

  it('handles full opacity', () => {
    expect(rgba('#10b981', 1)).toBe('rgba(16, 185, 129, 1)');
  });
});

describe('deriveAccentPalette', () => {
  it('returns the default emerald palette', () => {
    const palette = deriveAccentPalette();
    expect(palette.base).toBe('#10b981');
    expect(palette.hover).toBeDefined();
    expect(palette.active).toBeDefined();
    expect(palette.subtle).toBeDefined();
    expect(palette.alpha).toBeDefined();
    expect(palette.secondary).toBeDefined();
  });

  it('uses dark fg text for light backgrounds', () => {
    // White luminance = 255 > 160 → light, so fg should be dark
    const palette = deriveAccentPalette('#ffffff');
    expect(palette.fg).toBe('#0a0a0a');
  });

  it('uses light fg text for dark backgrounds', () => {
    // Black luminance = 0 < 160 → dark, so fg should be white
    const palette = deriveAccentPalette('#000000');
    expect(palette.fg).toBe('#ffffff');
  });

  it('derives hover and active as darker variants', () => {
    const palette = deriveAccentPalette('#ff0000');
    // hover is base darkened by 10%
    expect(palette.hover).toBe(darken('#ff0000', 0.1));
    expect(palette.active).toBe(darken('#ff0000', 0.2));
  });

  it('returns consistent dim and alpha values', () => {
    const palette = deriveAccentPalette('#10b981');
    expect(palette.dim).toContain('rgba');
    expect(palette.alpha).toContain('rgba');
  });
});

describe('applyAccentPalette', () => {
  beforeEach(() => {
    // Reset style properties before each test
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

  it('sets CSS custom properties on documentElement', () => {
    const palette = deriveAccentPalette('#ff5722');
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
