import { describe, expect, it } from 'vitest';
import {
  hexToRgb,
  rgbToHex,
  lighten,
  darken,
  rgba,
  deriveAccentPalette,
} from '@/utils/color';

describe('hexToRgb', () => {
  it('parses 6-digit hex', () => {
    expect(hexToRgb('#ff6600')).toEqual({ r: 255, g: 102, b: 0 });
  });

  it('parses 3-digit hex', () => {
    expect(hexToRgb('#f60')).toEqual({ r: 255, g: 102, b: 0 });
  });

  it('parses without hash prefix', () => {
    expect(hexToRgb('ff6600')).toEqual({ r: 255, g: 102, b: 0 });
  });

  it('parses black', () => {
    expect(hexToRgb('#000000')).toEqual({ r: 0, g: 0, b: 0 });
  });

  it('parses white', () => {
    expect(hexToRgb('#ffffff')).toEqual({ r: 255, g: 255, b: 255 });
  });
});

describe('rgbToHex', () => {
  it('converts RGB to hex', () => {
    expect(rgbToHex(255, 102, 0)).toBe('#ff6600');
  });

  it('clamps out-of-range values', () => {
    expect(rgbToHex(300, -10, 128)).toBe('#ff0080');
  });

  it('pads single-digit hex values', () => {
    expect(rgbToHex(0, 15, 5)).toBe('#000f05');
  });
});

describe('lighten', () => {
  it('lightens toward white at amount=0.5', () => {
    expect(lighten('#000000', 0.5)).toBe('#808080');
  });

  it('returns original at amount=0', () => {
    expect(lighten('#ff0000', 0)).toBe('#ff0000');
  });

  it('returns white at amount=1', () => {
    expect(lighten('#000000', 1)).toBe('#ffffff');
  });
});

describe('darken', () => {
  it('darkens toward black at amount=0.5', () => {
    expect(darken('#ffffff', 0.5)).toBe('#808080');
  });

  it('returns original at amount=0', () => {
    expect(darken('#ff0000', 0)).toBe('#ff0000');
  });

  it('returns black at amount=1', () => {
    expect(darken('#ffffff', 1)).toBe('#000000');
  });
});

describe('rgba', () => {
  it('formats rgba string', () => {
    expect(rgba('#ff6600', 0.5)).toBe('rgba(255, 102, 0, 0.5)');
  });

  it('handles full opacity', () => {
    expect(rgba('#000', 1)).toBe('rgba(0, 0, 0, 1)');
  });
});

describe('deriveAccentPalette', () => {
  it('returns all palette keys', () => {
    const palette = deriveAccentPalette('#10b981');
    expect(palette).toHaveProperty('base', '#10b981');
    expect(palette).toHaveProperty('hover');
    expect(palette).toHaveProperty('active');
    expect(palette).toHaveProperty('subtle');
    expect(palette).toHaveProperty('fg');
    expect(palette).toHaveProperty('dim');
    expect(palette).toHaveProperty('alpha');
    expect(palette).toHaveProperty('secondary');
  });

  it('uses dark fg for light backgrounds', () => {
    const palette = deriveAccentPalette('#ffffff');
    expect(palette.fg).toBe('#0a0a0a');
  });

  it('uses white fg for dark backgrounds', () => {
    const palette = deriveAccentPalette('#000000');
    expect(palette.fg).toBe('#ffffff');
  });

  it('returns consistent palette for the same input', () => {
    const a = deriveAccentPalette('#ff6600');
    const b = deriveAccentPalette('#ff6600');
    expect(a).toEqual(b);
  });
});
