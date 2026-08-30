// @vitest-environment jsdom
// Unit tests for the admin dashboard's pure helpers (H2 hardening).
// The helpers live in public/admin/admin-utils.js — a UMD module that
// exports for Node/vitest and defines window.AdminUtils in the browser.
import { describe, expect, it } from 'vitest';
import utils from '../../public/admin/admin-utils.js';

describe('admin-utils escapeHtml', () => {
  it('escapes HTML entities', () => {
    expect(utils.escapeHtml('<script>&"\'')).toBe('&lt;script&gt;&amp;&quot;&#39;');
  });

  it('passes through safe strings', () => {
    expect(utils.escapeHtml('plain text 123')).toBe('plain text 123');
  });

  it('handles non-strings', () => {
    expect(utils.escapeHtml(null)).toBe('null');
    expect(utils.escapeHtml(undefined)).toBe('undefined');
    expect(utils.escapeHtml(42)).toBe('42');
  });
});

describe('admin-utils formatting', () => {
  it('fmtIdr formats IDR with Rp prefix', () => {
    expect(utils.fmtIdr(1000000)).toBe('Rp 1.000.000');
  });

  it('fmtUsd formats USD with 2 decimals', () => {
    expect(utils.fmtUsd(12.5)).toBe('$12.50');
    expect(utils.fmtUsd(0)).toBe('$0.00');
  });
});

describe('admin-utils statusPill', () => {
  it('maps known statuses to pill classes', () => {
    expect(utils.statusPill('active').className).toBe('pill pill-ok');
    expect(utils.statusPill('expired').className).toBe('pill pill-bad');
    expect(utils.statusPill('grace_period').className).toBe('pill pill-warn');
  });

  it('falls back to muted for unknown statuses', () => {
    expect(utils.statusPill('weird').className).toBe('pill pill-muted');
  });
});

describe('admin-utils svgChart', () => {
  it('returns empty state for empty data (M1 guard)', () => {
    expect(utils.svgChart('id', [], ['idr'])).toContain('chart-empty');
    expect(utils.svgChart('id', null, ['idr'])).toContain('chart-empty');
    expect(utils.svgChart('id', undefined, ['idr'])).toContain('chart-empty');
  });

  it('returns empty state for all-NaN data', () => {
    const data = [{ month: '2026-01', idr: 'abc' }];
    expect(utils.svgChart('id', data, ['idr'])).toContain('chart-empty');
  });

  it('renders an svg with a path for valid data', () => {
    const data = [
      { month: '2026-01', idr: 100 },
      { month: '2026-02', idr: 200 },
    ];
    const svg = utils.svgChart('id', data, ['idr'], { area: true });
    expect(svg).toContain('<svg');
    expect(svg).toContain('chart-line');
    // X-axis labels are month.slice(5) → "01" / "02".
    expect(svg).toContain('01');
    expect(svg).toContain('02');
  });
});

describe('admin-utils svgDonut', () => {
  it('returns empty state for empty data (M1 guard)', () => {
    expect(utils.svgDonut('id', [], 'tier', 'count').svg).toContain('chart-empty');
    expect(utils.svgDonut('id', null, 'tier', 'count').svg).toContain('chart-empty');
  });

  it('returns empty state for zero total', () => {
    const data = [{ tier: 'plus', count: 0 }];
    expect(utils.svgDonut('id', data, 'tier', 'count').svg).toContain('chart-empty');
  });

  it('renders slices + escaped legend labels', () => {
    const data = [
      { tier: 'plus', count: 3 },
      { tier: 'pro', count: 1 },
    ];
    const { svg, legend } = utils.svgDonut('id', data, 'tier', 'count');
    expect(svg).toContain('<svg');
    expect(svg).toContain('<path');
    expect(legend).toContain('plus');
    expect(legend).toContain('pro');
    expect(legend).toContain('75%');
    expect(legend).toContain('25%');
  });

  it('escapes legend labels with HTML entities', () => {
    const data = [{ tier: '<b>bad</b>', count: 1 }];
    const { legend } = utils.svgDonut('id', data, 'tier', 'count');
    expect(legend).not.toContain('<b>bad</b>');
    expect(legend).toContain('&lt;b&gt;bad&lt;/b&gt;');
  });
});

describe('admin-utils kpiC', () => {
  it('builds a kpi card with label + value + sub', () => {
    const k = utils.kpiC('Revenue', 'Rp 1M', 'this month');
    expect(k.className).toBe('kpi');
    expect(k.querySelector('.kpi-label').textContent).toBe('Revenue');
    expect(k.querySelector('.kpi-value').textContent).toBe('Rp 1M');
    expect(k.querySelector('.kpi-sub').textContent).toBe('this month');
  });

  it('renders an icon box with the given class when icon is provided', () => {
    const k = utils.kpiC('Devices', '3', null, '<svg></svg>', 'kpi-icon-green');
    const icon = k.querySelector('.kpi-icon');
    expect(icon).not.toBeNull();
    expect(icon.className).toContain('kpi-icon-green');
    expect(icon.querySelector('svg')).not.toBeNull();
  });
});

describe('admin-utils tableCard', () => {
  it('builds a card with headers and rows', () => {
    const t = utils.tableCard('Tenants', ['Email', 'Tier'], [['a@b.com', 'pro'], ['c@d.com', 'plus']]);
    expect(t.className).toContain('card');
    expect(t.querySelector('h2').textContent).toBe('Tenants');
    expect(t.querySelectorAll('th').length).toBe(2);
    expect(t.querySelectorAll('th')[0].textContent).toBe('Email');
    expect(t.querySelectorAll('tbody tr').length).toBe(2);
    expect(t.querySelectorAll('tbody td')[0].textContent).toBe('a@b.com');
  });

  it('renders an empty state for zero rows', () => {
    const t = utils.tableCard('Tenants', ['Email'], []);
    expect(t.textContent).toContain('No data.');
    expect(t.querySelector('table')).toBeNull();
  });
});
