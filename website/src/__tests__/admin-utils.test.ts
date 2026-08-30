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

describe('admin-utils i18n (H3)', () => {
  it('t() returns the localized string for known keys', () => {
    expect(utils.t('kpi.totalUsers')).toBe('Total Users');
    expect(utils.t('table.expiringSoon')).toBe('Expiring Soon (within 30 days)');
    expect(utils.t('tenant.renew365')).toBe('Renew +365d');
  });

  it('t() falls back to the key itself for missing keys', () => {
    expect(utils.t('missing.key.here')).toBe('missing.key.here');
  });

  it('exposes the full STRINGS dictionary', () => {
    expect(utils.STRINGS).toBeDefined();
    expect(Object.keys(utils.STRINGS).length).toBeGreaterThan(60);
  });

  it('covers the login page strings (admin + dashboard share the table)', () => {
    expect(utils.t('login.sendCode')).toBe('Send Verification Code');
    expect(utils.t('login.enterEmail')).toBe('Please enter your email address');
    expect(utils.t('login.couldNotConnect')).toBe('Could not connect to authentication server');
    expect(utils.t('login.resendIn')).toBe('Resend code in ');
  });
});

describe('admin-utils API auth helpers (H1)', () => {
  it('isAuthDenied classifies 401/403', () => {
    expect(utils.isAuthDenied(401)).toBe(true);
    expect(utils.isAuthDenied(403)).toBe(true);
    expect(utils.isAuthDenied(200)).toBe(false);
    expect(utils.isAuthDenied(500)).toBe(false);
    expect(utils.isAuthDenied(404)).toBe(false);
  });

  it('authDeniedError marks the error with authDenied + path', () => {
    const err = utils.authDeniedError('/api/v1/admin/stats') as Error & { authDenied?: boolean };
    expect(err.message).toContain('/api/v1/admin/stats');
    expect(err.authDenied).toBe(true);
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

// ── Bug hunt 2026-08-30: the i18n refactor (#73) replaced literals with
// t(...) inside callbacks whose parameter was ALSO named t — calling a
// tenant object as a function threw TypeError and killed the whole
// Tenants tab. These tests pin the extracted helpers against that class
// of shadowing regression.

describe('admin-utils tenantRow (B1: t() shadowing regression)', () => {
  const tenant = {
    id: 't1',
    email: 'a@b.c',
    status: 'active',
    license: { key: 'OZ-KEY' },
    subscription: { tierKey: 'pro' },
    created: '2026-08-01T10:00:00Z',
  };

  it('renders all six cells without crashing', () => {
    const row = utils.tenantRow(tenant, () => {});
    const cells = row.querySelectorAll('td');
    expect(cells.length).toBe(6);
    expect(cells[0].textContent).toBe('a@b.c');
    expect(cells[2].textContent).toBe('OZ-KEY');
    expect(cells[3].textContent).toBe('pro');
    expect(cells[4].textContent).toBe('2026-08-01');
  });

  it('labels the action button via i18n and wires the click to the tenant id', () => {
    const clicks: string[] = [];
    const row = utils.tenantRow(tenant, (id: string) => clicks.push(id));
    const btn = row.querySelector('button')!;
    expect(btn.textContent).toBe('Details');
    btn.click();
    expect(clicks).toEqual(['t1']);
  });

  it('falls back to em-dashes for missing optional fields', () => {
    const row = utils.tenantRow({ id: 'x', status: 'active' }, () => {});
    const cells = row.querySelectorAll('td');
    expect(cells[0].textContent).toBe('—');
    expect(cells[2].textContent).toBe('—');
    expect(cells[4].textContent).toBe('—');
  });
});
