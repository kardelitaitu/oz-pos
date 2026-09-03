// @vitest-environment jsdom
// Unit tests for the admin dashboard's pure helpers (H2 hardening).
// The helpers live in public/admin/admin-utils.js — a UMD module that
// exports for Node/vitest and defines window.AdminUtils in the browser.
import { afterEach, describe, expect, it, vi } from 'vitest';
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

  it('does not crash when a row is missing the month label (B5)', () => {
    // The M1 guard protects VALUES (NaN filtering) but the x-axis label
    // code did d.month.slice(5) unguarded — one row without a month
    // (new bucket shape, partial API payload) threw TypeError and killed
    // the whole dashboard render, not just the chart.
    const data = [
      { month: '2026-01', idr: 100 },
      { idr: 200 },
      { month: null, idr: 300 },
    ];
    const svg = utils.svgChart('id', data, ['idr']);
    expect(svg).toContain('<svg');
    expect(svg).toContain('01');
  });

  it('wide variant stretches the canvas so text is not upscaled', () => {
    const data = Array.from({ length: 12 }, (_, i) => ({ month: `2026-${String(i + 1).padStart(2, '0')}`, idr: (i + 1) * 100 }));
    const svg = utils.svgChart('rev', data, ['idr'], { area: true, wide: true });
    expect(svg).toContain('viewBox="0 0 1280 230"');
    // A label under every point on the wide canvas (12 months → 12 x-labels).
    expect(svg.match(/text-anchor="middle"/g)?.length).toBe(12);
  });

  it('sourceKey: renders a solid path when all months are verified', () => {
    const data = [
      { month: '2026-01', idr: 100, source: 'paddle_webhook' },
      { month: '2026-02', idr: 200, source: 'midtrans_webhook' },
    ];
    const svg = utils.svgChart('id', data, ['idr'], { area: true, sourceKey: 'source' });
    // No dasharray attribute on the solid segments.
    expect(svg).not.toContain('stroke-dasharray');
    // Area fill present (both segments are non-estimate).
    expect(svg).toContain('opacity=".1"');
  });

  it('sourceKey: dashes estimate months and omits their area fill', () => {
    const data = [
      { month: '2026-01', idr: 100, source: 'paddle_webhook' },
      { month: '2026-02', idr: 200, source: 'estimate' },
      { month: '2026-03', idr: 300, source: 'paddle_webhook' },
    ];
    const svg = utils.svgChart('id', data, ['idr'], { area: true, sourceKey: 'source' });
    // The estimate month segment should have a dashed stroke.
    expect(svg).toContain('stroke-dasharray="5 4"');
    // Area fill should still be present (the two solid segments).
    expect(svg).toContain('opacity=".1"');
  });

  it('sourceKey: falls back to estimate when sourceKey is missing on a row', () => {
    const data = [
      { month: '2026-01', idr: 100, source: 'paddle_webhook' },
      { month: '2026-02', idr: 200 }, // no source → treated as estimate
    ];
    const svg = utils.svgChart('id', data, ['idr'], { area: true, sourceKey: 'source' });
    expect(svg).toContain('stroke-dasharray="5 4"');
  });
});

describe('admin-utils fmtMonthTick (year-boundary x-labels)', () => {
  it('keeps plain month labels within a single year', () => {
    expect(utils.fmtMonthTick('2026-03', '2026')).toEqual({ label: '03', year: '2026' });
    expect(utils.fmtMonthTick('2026-11', '2026')).toEqual({ label: '11', year: '2026' });
  });

  it('adds a year suffix when the year changes (Dec → Jan)', () => {
    // Previous emitted tick was Dec 2025; the next is Jan 2026.
    const jan = utils.fmtMonthTick('2026-01', '2025');
    expect(jan).toEqual({ label: '01/26', year: '2026' });
  });

  it('adds a year suffix on the first emitted tick (no prior year)', () => {
    expect(utils.fmtMonthTick('2025-12', '')).toEqual({ label: '12/25', year: '2025' });
  });

  it('keeps the year across the first tick until a boundary', () => {
    // Simulate a 13-month window: first tick Nov 2025, then Dec 2025
    // (same year), then Jan 2026 (boundary).
    const nov = utils.fmtMonthTick('2025-11', '');
    const dec = utils.fmtMonthTick('2025-12', nov.year);
    const jan = utils.fmtMonthTick('2026-01', dec.year);
    expect(nov.label).toBe('11/25');
    expect(dec.label).toBe('12');
    expect(jan.label).toBe('01/26');
  });

  it('survives rows without a month (B5-style)', () => {
    expect(utils.fmtMonthTick(null, '2026')).toEqual({ label: '', year: '2026' });
    expect(utils.fmtMonthTick(undefined, '2026')).toEqual({ label: '', year: '2026' });
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

  it('renders the total count in the center of the donut', () => {
    const data = [
      { tier: 'plus', count: 3 },
      { tier: 'pro', count: 1 },
    ];
    const { svg } = utils.svgDonut('id', data, 'tier', 'count');
    // Total = 4; center text should render "4" (toLocaleString form).
    expect(svg).toContain('>4<');
    // The text element sits inside the SVG at the center.
    expect(svg).toContain('text-anchor="middle"');
    expect(svg).toContain('font-size="17"');
  });

  it('shows a single entry count for 100% donuts', () => {
    const { svg } = utils.svgDonut('id', [{ tier: 'free', count: 5 }], 'tier', 'count');
    expect(svg).toContain('>5<');
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

describe('admin-utils statC (design-language tinted stat card)', () => {
  it('builds a tinted stat card with value + label + sub', () => {
    const s = utils.statC('Orders Today', '142', 'caption', 'primary');
    expect(s.className).toBe('stat stat--primary');
    expect(s.querySelector('.stat-value').textContent).toBe('142');
    expect(s.querySelector('.stat-label').textContent).toBe('Orders Today');
    expect(s.querySelector('.stat-sub').textContent).toBe('caption');
  });

  it('maps every known variant to its class', () => {
    for (const v of ['primary', 'success', 'warning', 'danger', 'info']) {
      expect(utils.statC('L', '1', '', v).className).toBe('stat stat--' + v);
    }
  });

  it('falls back to primary for unknown variants and omits empty sub', () => {
    const s = utils.statC('X', '1', '', 'chartreuse');
    expect(s.className).toBe('stat stat--primary');
    expect(s.querySelector('.stat-sub')).toBeNull();
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
    subscription: { tierKey: 'pro', expiresAt: '2027-08-01T00:00:00Z' },
    created: '2026-08-01T10:00:00Z',
  };

  it('renders five cells: email, status, merged license/tier, created, action', () => {
    const row = utils.tenantRow(tenant, () => {});
    const cells = row.querySelectorAll('td');
    expect(cells.length).toBe(5);
    expect(cells[0].textContent).toBe('a@b.c');
    // merged "[tier] date expired" format, date = subscription expiry
    expect(cells[2].textContent).toBe('[pro] 2027-08-01');
    expect(cells[2].getAttribute('title')).toBe('[pro] 2027-08-01 · OZ-KEY');
    expect(cells[3].textContent).toBe('2026-08-01');
  });

  it('expiry falls back to the license when the subscription has none', () => {
    const row = utils.tenantRow({
      id: 't2', status: 'active',
      license: { key: 'OZ-L', tierKey: 'plus', expiresAt: '2026-12-25' },
      subscription: {},
    }, () => {});
    const cells = row.querySelectorAll('td');
    expect(cells[2].textContent).toBe('[plus] 2026-12-25');
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
    expect(cells[3].textContent).toBe('—');
  });
});

describe('admin-utils tenantDetailRows (B2: t() shadowing regression)', () => {
  const data = {
    tenant: { status: 'active', emailVerified: true, created: '2026-08-01T10:00:00Z' },
    license: { key: 'OZ-KEY' },
    subscription: { tierKey: 'pro', status: 'active', expiresAt: '2027-08-01' },
    devices: [{ id: 'd1' }, { id: 'd2' }],
  };

  it('builds the 9 key/value rows (phone added, devices last) without crashing', () => {
    const rows = utils.tenantDetailRows(data);
    expect(rows.length).toBe(9);
    // B16 superseded the raw-enum expectation: status rows carry labels.
    expect(rows[0]).toEqual(['Status', 'Active']);
    expect(rows[1]).toEqual(['Email verified', '✓']);
    expect(rows[2]).toEqual(['Phone', '—']);
    expect(rows[3]).toEqual(['Created', '2026-08-01']);
    expect(rows[4]).toEqual(['License key', 'OZ-KEY']);
    expect(rows[5]).toEqual(['Tier', 'pro']);
    expect(rows[6]).toEqual(['Subscription status', 'Active']);
    expect(rows[7]).toEqual(['Expires', '2027-08-01']);
    expect(rows[8]).toEqual(['Devices', 2]);
  });

  it('includes phone when present, grace only when set', () => {
    const rows = utils.tenantDetailRows({
      tenant: { status: 'active', emailVerified: true, created: '2026-08-01', phone: '+62 812-3456-7890' },
      license: { key: 'OZ-K' },
      subscription: { tierKey: 'pro', status: 'active', expiresAt: '2027-08-01' },
      devices: [],
    });
    const labels = (rows as any[]).map((r) => r?.[0]);
    expect(labels).toContain('Phone');
    expect(labels).not.toContain('Grace until');
    // position: right after Email verified
    expect(labels.indexOf('Phone')).toBe(labels.indexOf('Email verified') + 1);
  });

  it('shows the grace row when graceUntil is set', () => {
    const rows = utils.tenantDetailRows({
      tenant: { status: 'active' },
      subscription: { tierKey: 'pro', graceUntil: '2026-09-10' },
    });
    const labels = (rows as any[]).map((r) => r?.[0]);
    expect(labels).toContain('Grace until');
    expect((rows as any[]).find((r) => r?.[0] === 'Grace until')?.[1]).toBe('2026-09-10');
  });

  it('handles a fully empty payload with em-dash values', () => {
    const rows = utils.tenantDetailRows({});
    expect(rows.length).toBe(9);
    expect(rows[0]?.[1]).toBe('—');
    expect(rows[8]?.[1]).toBe(0);
  });
});

describe('admin-utils revokeConfirmModal (confirm-by-email guard)', () => {
  const setup = () => {
    const confirmSpy = vi.fn();
    const { box, cancelBtn } = utils.revokeConfirmModal('Owner@Example.com ', confirmSpy);
    document.body.appendChild(box);
    return { box, cancelBtn, confirmSpy };
  };
  const teardown = () => document.querySelectorAll('.modal').forEach(m => m.remove());

  afterEach(teardown);

  it('confirm stays disabled until the typed email matches (trim+case-insensitive)', () => {
    const { box, confirmSpy } = setup();
    const input = box.querySelector('input') as HTMLInputElement;
    const confirm = [...box.querySelectorAll('button')].find(b => b.textContent === 'Revoke') as HTMLButtonElement;
    expect(confirm.disabled).toBe(true);
    input.value = 'wrong@b.c';
    input.dispatchEvent(new Event('input'));
    expect(confirm.disabled).toBe(true);
    input.value = 'owner@example.com';
    input.dispatchEvent(new Event('input'));
    expect(confirm.disabled).toBe(false);
    confirm.click();
    expect(confirmSpy).toHaveBeenCalledTimes(1);
  });

  it('shows a live mismatch hint while typing, hides it on match/clear', () => {
    const { box } = setup();
    const input = box.querySelector('input') as HTMLInputElement;
    const err = box.querySelector('p[style*="danger"]') as HTMLElement;
    input.value = 'wron';
    input.dispatchEvent(new Event('input'));
    expect(err.style.display).toBe('block');
    input.value = 'owner@example.com';
    input.dispatchEvent(new Event('input'));
    expect(err.style.display).toBe('none');
    input.value = '';
    input.dispatchEvent(new Event('input'));
    expect(err.style.display).contains('none');
  });

  it('cancel button is returned for the caller to wire', () => {
    const { box, cancelBtn } = setup();
    expect(box.contains(cancelBtn)).toBe(true);
    expect(cancelBtn.textContent).toBe('Cancel');
  });

  it('opts reuse the gate for the delete flow (title, hint, label, cascade warn)', () => {
    const confirmSpy = vi.fn();
    const { box } = utils.revokeConfirmModal('del@x.id', confirmSpy, {
      title: 'Delete tenant permanently',
      hint: 'Cannot be undone. Type the tenant email to confirm: ',
      confirmLabel: 'Delete permanently',
      extraWarn: 'All devices and subscriptions are deleted.',
    });
    document.body.appendChild(box);
    expect(box.querySelector('h3')!.textContent).toBe('Delete tenant permanently');
    expect(box.textContent).toContain('Cannot be undone.');
    expect(box.textContent).toContain('All devices and subscriptions are deleted.');
    const confirm = [...box.querySelectorAll('button')].find(b => b.textContent === 'Delete permanently') as HTMLButtonElement;
    expect(confirm.disabled).toBe(true);
    const input = box.querySelector('input') as HTMLInputElement;
    input.value = 'del@x.id';
    input.dispatchEvent(new Event('input'));
    expect(confirm.disabled).toBe(false);
    confirm.click();
    expect(confirmSpy).toHaveBeenCalledTimes(1);
  });
});

describe('admin-utils svgBarChart (B3: churn chart read the wrong field)', () => {
  // The server's churnPerMonth rows are monthBucket{Month, Churn} with
  // count left at Go's zero value — a chart that reads d.count renders
  // permanently-zero bars. valueKey must be honored.
  const churnData = [
    { month: '2026-01', count: 0, churn: 5 },
    { month: '2026-02', count: 0, churn: 10 },
  ];

  it('scales bars by the requested valueKey, not a hardcoded field', () => {
    const svg = utils.svgBarChart('churn', churnData, { valueKey: 'churn', color: 'var(--bad)' });
    // max=10 → full height 141; 5 → half height 70.5.
    expect(svg).toContain('height="141"');
    expect(svg).toContain('height="70.5"');
    expect(svg).not.toContain('NaN');
  });

  it('defaults to count for signup-shaped data', () => {
    const svg = utils.svgBarChart('signups', [
      { month: '2026-01', count: 4 },
      { month: '2026-02', count: 8 },
    ], { color: 'var(--accent)' });
    expect(svg).toContain('height="141"');
    expect(svg).toContain('height="70.5"');
  });

  it('renders the empty state instead of Infinity geometry for zero rows', () => {
    expect(utils.svgBarChart('x', [], { valueKey: 'count' })).toContain('chart-empty');
  });

  it('escapes month labels and survives missing months', () => {
    // Month labels are the slice AFTER 'YYYY-' — put the markup there.
    const svg = utils.svgBarChart('x', [{ count: 1 }, { count: 1, month: '2026-<b>' }], { valueKey: 'count' });
    expect(svg).not.toContain('<b>');
    expect(svg).toContain('&lt;b&gt;');
  });

  it('wide variant stretches the canvas so text is not upscaled', () => {
    const data = Array.from({ length: 12 }, (_, i) => ({ month: `2026-${String(i + 1).padStart(2, '0')}`, churn: i + 1 }));
    const narrow = utils.svgBarChart('c', data, { valueKey: 'churn' });
    const wide = utils.svgBarChart('c', data, { valueKey: 'churn', wide: true });
    expect(narrow).toContain('viewBox="0 0 620 200"');
    expect(wide).toContain('viewBox="0 0 1280 230"');
    expect(wide).not.toContain('max-height');
    // Max bar still reaches the same plot height (161) in both canvases.
    expect(wide).toContain('height="161"');
  });
});

describe('admin-utils svgStackedBars (provider revenue mix)', () => {  it('returns empty state for empty or missing data', () => {
    expect(utils.svgStackedBars('x', [], { stack: [{ key: 'a', color: 'red' }] })).toContain('chart-empty');
    expect(utils.svgStackedBars('x', null, { stack: [{ key: 'a', color: 'red' }] })).toContain('chart-empty');
    expect(utils.svgStackedBars('x', undefined, { stack: [{ key: 'a', color: 'red' }] })).toContain('chart-empty');
  });

  it('returns empty state when no stack segments are defined', () => {
    expect(utils.svgStackedBars('x', [{ month: '2026-01', a: 10 }], {})).toContain('chart-empty');
  });

  it('stacks two segments per month and renders the total label', () => {
    const data = [
      { month: '2026-01', paddleIdr: 60000, midtransIdr: 40000 }, // total 100000
      { month: '2026-02', paddleIdr: 30000, midtransIdr: 20000 }, // total 50000
    ];
    const svg = utils.svgStackedBars('mix', data, {
      stack: [
        { key: 'paddleIdr', color: 'var(--primary)' },
        { key: 'midtransIdr', color: 'var(--success)' },
      ],
    });
    // Two months → two month labels
    expect(svg.match(/text-anchor="middle" fill="var\(--muted\)"/g)?.length).toBe(2);
    // Total labels: month 1 = 100k, month 2 = 50k
    expect(svg).toContain('100000');
    expect(svg).toContain('50000');
  });

  it('skips the total label for zero-total (estimate) months', () => {
    const data = [
      { month: '2026-01', paddleIdr: 60000, midtransIdr: 40000 },
      { month: '2026-02', paddleIdr: 0, midtransIdr: 0 }, // estimate
    ];
    const svg = utils.svgStackedBars('mix', data, {
      stack: [{ key: 'paddleIdr', color: 'var(--primary)' }, { key: 'midtransIdr', color: 'var(--success)' }],
    });
    // Only one total label for the non-zero month; the estimate month has no
    // value label but still gets a month x-label.
    expect(svg.match(/font-weight="600"/g)?.length).toBe(1);
    expect(svg.match(/02/)).toBeTruthy();
  });

  it('renders both color segments in the correct order', () => {
    const data = [{ month: '2026-01', paddleIdr: 60000, midtransIdr: 40000 }];
    const svg = utils.svgStackedBars('mix', data, {
      stack: [{ key: 'paddleIdr', color: 'red' }, { key: 'midtransIdr', color: 'blue' }],
    });
    // Both rects with their respective fill colors
    expect(svg.match(/fill="red"/)).toBeTruthy();
    expect(svg.match(/fill="blue"/)).toBeTruthy();
  });
});

describe('admin-utils svgDonut single-slice (B4: invisible 100% donut)', () => {
  it('renders a visible full circle when one entry holds 100%', () => {
    // Common early deployment state: every tenant on one tier (or one
    // payment provider). A single SVG arc whose start point equals its end
    // point draws NOTHING (spec behavior) — the donut looked empty while
    // the legend claimed 100%. The full circle must be split into arcs.
    const { svg } = utils.svgDonut('id', [{ tier: 'free', count: 5 }], 'tier', 'count');
    expect(svg).toContain('<svg');
    const arcs = svg.match(/ A /g) || [];
    expect(arcs.length).toBeGreaterThanOrEqual(2);
  });

  it('multi-slice donuts keep one arc per slice', () => {
    const { svg } = utils.svgDonut('id', [
      { tier: 'free', count: 5 },
      { tier: 'pro', count: 5 },
    ], 'tier', 'count');
    expect((svg.match(/ A /g) || []).length).toBe(2);
  });
});

describe('admin-utils normalizeStats (B6: partial payload killed the dashboard)', () => {
  it('fills missing collections so render code never throws', () => {
    // admin.js did m.revenueTrend.forEach(...) and m.kpis.mrrUsd BEFORE
    // the chart guards could help — a stats payload missing any of those
    // fields (partial response, older server build, error body with 200)
    // threw TypeError and the dashboard showed nothing but a console
    // error. normalizeStats guarantees the shapes the render expects.
    const m = utils.normalizeStats({});
    expect(Array.isArray(m.revenueTrend)).toBe(true);
    expect(Array.isArray(m.subscriberGrowth)).toBe(true);
    expect(Array.isArray(m.tierDistribution)).toBe(true);
    expect(Array.isArray(m.providerSplit)).toBe(true);
    expect(Array.isArray(m.signupsPerMonth)).toBe(true);
    expect(Array.isArray(m.churnPerMonth)).toBe(true);
    expect(m.kpis).toBeTypeOf('object');
    expect(m.kpis.mrrUsd).toBeTypeOf('number');
    expect(m.kpis.totalUsers).toBeTypeOf('number');
  });

  it('keeps valid data untouched', () => {
    const src = {
      revenueTrend: [{ month: '2026-01', usd: 10 }],
      kpis: { mrrUsd: 42.5, totalUsers: 7, fxRate: 16000 },
    };
    const m = utils.normalizeStats(src);
    expect(m.revenueTrend).toEqual(src.revenueTrend);
    expect(m.kpis.mrrUsd).toBe(42.5);
    expect(m.kpis.totalUsers).toBe(7);
    expect(m.kpis.fxRate).toBe(16000);
  });

  it('coerces null-ish numeric kpis to 0', () => {
    const m = utils.normalizeStats({ kpis: { mrrUsd: null, totalUsers: undefined, arpuUsd: '12.5' } });
    expect(m.kpis.mrrUsd).toBe(0);
    expect(m.kpis.totalUsers).toBe(0);
    expect(m.kpis.arpuUsd).toBe(12.5);
  });

  it('coerces the provider monthly-gross kpis (provider-verified revenue)', () => {
    // monthlyGrossUsd/Idr come from the revenue_events webhook ledger; a
    // partial payload must never render "Rp NaN" in the hero card.
    const m = utils.normalizeStats({
      kpis: { monthlyGrossUsd: null, monthlyGrossIdr: undefined, grossSource: 'estimate' },
    });
    expect(m.kpis.monthlyGrossUsd).toBe(0);
    expect(m.kpis.monthlyGrossIdr).toBe(0);
    // grossSource is a string, passed through untouched.
    expect(m.kpis.grossSource).toBe('estimate');
  });

  it('coerces the refund kpis (revenue_adjustments ledger)', () => {
    const m = utils.normalizeStats({
      kpis: { monthlyRefundUsd: null, monthlyRefundIdr: '160000', lifetimeRefundUsd: 25.5, lifetimeRefundIdr: undefined },
    });
    expect(m.kpis.monthlyRefundUsd).toBe(0);
    expect(m.kpis.monthlyRefundIdr).toBe(160000);
    expect(m.kpis.lifetimeRefundUsd).toBe(25.5);
    expect(m.kpis.lifetimeRefundIdr).toBe(0);
  });

  it('keeps provider-verified per-month trend values and source labels', () => {
    const src = {
      revenueTrend: [
        { month: '2026-01', usd: 10, idr: 160000, source: 'paddle_webhook' },
        { month: '2026-02', usd: 9.3, idr: 149000, source: 'midtrans_webhook' },
        { month: '2026-03', usd: 5, source: 'estimate' },
      ],
    };
    const m = utils.normalizeStats(src);
    expect(m.revenueTrend).toEqual(src.revenueTrend);
    expect(m.revenueTrend[0].idr).toBe(160000);
    expect(m.revenueTrend[1].source).toBe('midtrans_webhook');
    // Estimate months may have no idr from the server — renderer derives it.
    expect(m.revenueTrend[2].idr).toBeUndefined();
  });

  it('passes through needsAttention array from the server', () => {
    const m = utils.normalizeStats({
      needsAttention: [
        { type: 'grace_period', email: 'a@b.com', tier: 'pro', detail: 'payment failed', at: '2026-09-15' },
      ],
    });
    expect(m.needsAttention).toHaveLength(1);
    expect(m.needsAttention[0].type).toBe('grace_period');
  });

  it('passes through recentRevenueEvents feed rows (currency-clean amounts)', () => {
    const m = utils.normalizeStats({
      recentRevenueEvents: [
        { email: 'x@y.com', provider: 'paddle', tier: 'pro', amountUsd: 42.5, amountIdr: 680000, created: '2026-09-01T08:00:00Z' },
      ],
    });
    expect(m.recentRevenueEvents).toHaveLength(1);
    expect(m.recentRevenueEvents[0].provider).toBe('paddle');
    expect(m.recentRevenueEvents[0].amountIdr).toBe(680000);
  });

  it('passes through trialFunnel array (trials vs paid per month)', () => {
    const m = utils.normalizeStats({
      trialFunnel: [
        { month: '2026-09', trials: 10, paid: 3 },
        { month: '2026-10', trials: 0, paid: 0 },
      ],
    });
    expect(m.trialFunnel).toHaveLength(2);
    expect(m.trialFunnel[0].trials).toBe(10);
    expect(m.trialFunnel[0].paid).toBe(3);
  });

  it('tolerates non-object input entirely', () => {
    expect(() => utils.normalizeStats(null)).not.toThrow();
    expect(() => utils.normalizeStats(undefined)).not.toThrow();
    expect(Array.isArray(utils.normalizeStats(null).revenueTrend)).toBe(true);
  });
});

describe('admin-utils startLockoutCountdown (B7: racing 429 timers)', () => {
  // login.js showLockoutCountdown created a NEW setInterval per 429
  // without clearing the previous one. Two rate-limited attempts (e.g.
  // request-otp then verify-otp) left two timers writing the same button:
  // the label flickered between both remaining values, and the FIRST
  // timer to expire re-enabled the button while the other still counted
  // down — then kept overwriting the restored label.
  const fmt = (s: number) => `Try again in ${s}s`;
  const restore = () => 'Send Verification Code';

  it('replaces the previous countdown instead of racing it', () => {
    vi.useFakeTimers();
    try {
      const btn = document.createElement('button');
      // First 429: retry_after 60. User retries anyway, second 429 says
      // 120 — the real lockout. The stale 60s timer must NOT re-enable
      // the button at t=60.
      utils.startLockoutCountdown(btn, 60, fmt, restore);
      utils.startLockoutCountdown(btn, 120, fmt, restore);
      vi.advanceTimersByTime(60000);
      expect(btn.disabled).toBe(true);
      expect(btn.textContent).toBe('Try again in 60s');
    } finally {
      vi.useRealTimers();
    }
  });

  it('enables the button and restores the label exactly once at zero', () => {
    vi.useFakeTimers();
    try {
      const btn = document.createElement('button');
      utils.startLockoutCountdown(btn, 60, fmt, restore);
      utils.startLockoutCountdown(btn, 2, fmt, restore);
      vi.advanceTimersByTime(2000);
      expect(btn.disabled).toBe(false);
      expect(btn.textContent).toBe('Send Verification Code');
      // The superseded 60s timer must not zombie-rewrite the label.
      vi.advanceTimersByTime(1000);
      expect(btn.textContent).toBe('Send Verification Code');
    } finally {
      vi.useRealTimers();
    }
  });
});

describe('admin-utils fetchFxRate (B10: unbounded hang on a dead FX API)', () => {
  // admin.js fetchFxRate awaited an un-timed fetch. When the stats payload
  // carried no fxRate (the server's own FX lookup failed -> falsy 0), the
  // dashboard fell into this path and hung on a captive-portal/firewalled
  // er-api.com for the browser's full connect timeout — skeleton forever.
  it('resolves a live rate from a good payload', async () => {
    const fetchImpl = async () => ({ json: async () => ({ rates: { IDR: 16450.5 } }) });
    const r = await utils.fetchFxRate(fetchImpl, 5000);
    expect(r.live).toBe(true);
    expect(r.rate).toBe(16450.5);
    expect(typeof r.updatedAt).toBe('string');
  });

  it('reports not-live for a malformed payload', async () => {
    const fetchImpl = async () => ({ json: async () => ({ rates: {} }) });
    const r = await utils.fetchFxRate(fetchImpl, 5000);
    expect(r).toEqual({ rate: null, updatedAt: '', live: false });
  });

  it('reports not-live when the fetch rejects', async () => {
    const fetchImpl = async () => { throw new Error('network'); };
    const r = await utils.fetchFxRate(fetchImpl, 5000);
    expect(r.live).toBe(false);
  });

  it('gives up at the timeout instead of hanging forever', async () => {
    // A fetch that never settles — the old code awaited this forever.
    // Real timers: AbortSignal.timeout is a native primitive that vitest
    // fake timers do not control; 50ms keeps the test fast.
    const fetchImpl = (_url: string, opts?: { signal?: AbortSignal }) =>
      new Promise((_resolve, reject) => {
        opts?.signal?.addEventListener('abort', () => reject(new Error('aborted')));
      });
    const r = await utils.fetchFxRate(fetchImpl, 50);
    expect(r.live).toBe(false);
    expect(r.rate).toBeNull();
  });
});

describe('admin-utils mountModal (B11: ESC listener leak)', () => {
  // The old inline modal code in admin.js registered a document keydown
  // handler per open, but only the ESC path ever removed it. Closing via
  // the Close button or a backdrop click left the listener attached —
  // every modal open without an ESC permanently added one.
  const trackKeydown = () => {
    let active = 0;
    const add = document.addEventListener.bind(document);
    const rm = document.removeEventListener.bind(document);
    document.addEventListener = ((type: string, ...rest: any[]) => {
      if (type === 'keydown') active++;
      return (add as any)(type, ...rest);
    }) as typeof document.addEventListener;
    document.removeEventListener = ((type: string, ...rest: any[]) => {
      if (type === 'keydown') active--;
      return (rm as any)(type, ...rest);
    }) as typeof document.removeEventListener;
    return { active: () => active, restore: () => { document.addEventListener = add as any; document.removeEventListener = rm as any; } };
  };

  it('close() detaches the keydown listener (no leak)', () => {
    const t = trackKeydown();
    try {
      const root = document.createElement('div');
      const close = utils.mountModal(root, document.createElement('div'));
      // B28 added a second keydown listener (the focus trap) — the
      // invariant is "mount attaches, close detaches everything", not
      // the exact count. (The old toBe(1) threw before close(), which
      // leaked two listeners into the next test — the -2 cascade.)
      expect(t.active()).toBeGreaterThanOrEqual(1);
      close();
      expect(root.children.length).toBe(0);
      expect(t.active()).toBe(0);
    } finally {
      t.restore();
    }
  });

  it('ESC closes the modal and detaches the listener', () => {
    const t = trackKeydown();
    try {
      const root = document.createElement('div');
      utils.mountModal(root, document.createElement('div'));
      document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
      expect(root.children.length).toBe(0);
      expect(t.active()).toBe(0);
      // A second ESC must be a no-op (handler already removed itself).
      document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
      expect(root.children.length).toBe(0);
    } finally {
      t.restore();
    }
  });

  it('backdrop click closes and detaches', () => {
    const t = trackKeydown();
    try {
      const root = document.createElement('div');
      utils.mountModal(root, document.createElement('div'));
      const back = root.firstElementChild as HTMLElement;
      back.dispatchEvent(new MouseEvent('click', { bubbles: true }));
      expect(root.children.length).toBe(0);
      expect(t.active()).toBe(0);
    } finally {
      t.restore();
    }
  });

  it('clicks inside the dialog do not close it', () => {
    const root = document.createElement('div');
    const box = document.createElement('div');
    utils.mountModal(root, box);
    box.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    expect(root.children.length).toBe(1);
  });
});

describe('admin-utils fetchWithTimeout (B12: api() had the same unbounded hang as the FX fetch)', () => {
  // Every admin data call goes through api(), which awaited two un-timed
  // fetches (/__oz/session and the license API). A hung license-server
  // connection left renderDashboard/renderTenants/renderHealth pending
  // forever — skeleton, no retry UI, no console error.

  it('passes through a normal response and forwards opts', async () => {
    let seen: any = null;
    const fetchImpl = async (_u: string, o?: any) => { seen = o; return { ok: true, status: 200 }; };
    const res: any = await utils.fetchWithTimeout(fetchImpl, 'https://x/api', { method: 'POST' }, 5000);
    expect(res.ok).toBe(true);
    expect(seen.method).toBe('POST');
    expect(seen.signal).toBeDefined();
  });

  it('propagates a rejected fetch', async () => {
    const fetchImpl = async () => { throw new Error('down'); };
    await expect(utils.fetchWithTimeout(fetchImpl, 'https://x/api', {}, 5000)).rejects.toThrow('down');
  });

  it('rejects at the timeout instead of hanging forever', async () => {
    // Real timers: AbortSignal.timeout is native and not faked.
    const fetchImpl = (_u: string, opts?: { signal?: AbortSignal }) =>
      new Promise((_resolve, reject) => {
        opts?.signal?.addEventListener('abort', () => reject(new Error('aborted')));
      });
    await expect(utils.fetchWithTimeout(fetchImpl, 'https://x/api', {}, 50)).rejects.toThrow();
  });
});

describe('admin-utils exchangeUrlFrom + isLockoutActive (B13/B14: login flow)', () => {
  // B13: login.js did window.location.href = '/?code=' + body.code with no
  // guard — a 200 response missing the code sent the browser to
  // /?code=undefined, the worker's exchange failed, redirected back to
  // login: a silent loop with no error shown.
  it('builds the exchange URL from a valid code', () => {
    expect(utils.exchangeUrlFrom({ code: 'abc 123' })).toBe('/?code=abc%20123');
  });

  it('throws when the server returns no usable code', () => {
    expect(() => utils.exchangeUrlFrom({})).toThrow();
    expect(() => utils.exchangeUrlFrom({ code: '' })).toThrow();
    expect(() => utils.exchangeUrlFrom(null)).toThrow();
  });

  // B14: setAuthMode overwrote the login button label on every tab
  // switch — during an active 429 lockout that left a disabled button
  // labelled "Send Verification Code" (the countdown text flickering back
  // a second later). The mode switch must respect an active lockout.
  it('isLockoutActive tracks the countdown lifecycle', () => {
    vi.useFakeTimers();
    try {
      const btn = document.createElement('button');
      expect(utils.isLockoutActive(btn)).toBe(false);
      utils.startLockoutCountdown(btn, 5, (s) => `${s}`, () => 'done');
      expect(utils.isLockoutActive(btn)).toBe(true);
      vi.advanceTimersByTime(5000);
      expect(utils.isLockoutActive(btn)).toBe(false);
    } finally {
      vi.useRealTimers();
    }
  });

  it('isLockoutActive tolerates a missing button', () => {
    expect(utils.isLockoutActive(null)).toBe(false);
    expect(utils.isLockoutActive(undefined)).toBe(false);
  });
});

describe('admin-utils statusPill labels (B16: raw enum text in the UI)', () => {
  // The server's status enum (handler_test.go SelectField values) leaked
  // straight into the UI: pills and the detail modal showed 'grace_period'
  // while every other label in the admin SPA is i18n'd via STRINGS.
  it('renders human labels for the server status enum', () => {
    expect(utils.statusPill('grace_period').textContent).toBe('Grace Period');
    expect(utils.statusPill('active').textContent).toBe('Active');
    expect(utils.statusPill('revoked').textContent).toBe('Revoked');
    expect(utils.statusPill('paused').textContent).toBe('Paused');
    expect(utils.statusPill('expired').textContent).toBe('Expired');
  });

  it('keeps the class mapping intact', () => {
    expect(utils.statusPill('grace_period').className).toContain('pill-warn');
    expect(utils.statusPill('active').className).toContain('pill-ok');
    expect(utils.statusPill('revoked').className).toContain('pill-bad');
  });

  it('unknown status falls back to the raw value, never a missing-key string', () => {
    expect(utils.statusPill('weird_state').textContent).toBe('weird_state');
    expect(utils.statusPill('').textContent).toBe('—');
  });

  it('detail modal rows use the same labels', () => {
    const rows = utils.tenantDetailRows({ tenant: { status: 'grace_period' } });
    expect(rows[0]?.[1]).toBe('Grace Period');
  });
});

describe('admin-utils createSeqGuard (B15: stale list responses overwrote newer ones)', () => {
  // renderTenants awaited api() without tracking which request was
  // newest: click page 2 then quickly page 3 — if page 2's response
  // arrived last it replaced page 3's rows while the pagination header
  // still said "Page 3 of N". Last-arrival-wins instead of last-click-wins.
  it('next() increases and isCurrent() accepts only the newest id', () => {
    const g = utils.createSeqGuard();
    const a = g.next();
    const b = g.next();
    expect(b).toBeGreaterThan(a);
    expect(g.isCurrent(b)).toBe(true);
    expect(g.isCurrent(a)).toBe(false);
  });

  it('out-of-order responses: only the newest request renders', async () => {
    const g = utils.createSeqGuard();
    const rendered: number[] = [];
    const fetchPage = async (page: number, delay: number) => {
      const s = g.next();
      await new Promise((r) => setTimeout(r, delay));
      if (!g.isCurrent(s)) return;
      rendered.push(page);
    };
    // Page 2 (slow) requested first, page 3 (fast) second: page 2 must
    // be discarded even though it arrives after page 3 rendered.
    await Promise.all([fetchPage(2, 60), fetchPage(3, 5)]);
    expect(rendered).toEqual([3]);
  });
});

describe('admin-utils startCountdown (B18: OTP cooldown lifecycle)', () => {
  // login.js startOtpCooldown kept its timer in a module global and
  // setAuthMode('password') HID the cooldown element without touching the
  // timer — switching back to the OTP tab never re-showed it, so a live
  // 60s resend cooldown ran invisibly and the user walked into a 429.
  // The countdown now lives on the node (same tracked-timer pattern as
  // startLockoutCountdown), so visibility can follow countdownActive().
  it('writes formatted text each second and fires onEnd at zero', () => {
    vi.useFakeTimers();
    try {
      const cd = document.createElement('span');
      let ended = 0;
      utils.startCountdown(cd, 3, (s) => `in ${s}`, () => { ended++; cd.textContent = 'ready'; });
      expect(cd.textContent).toBe('in 3');
      vi.advanceTimersByTime(2000);
      expect(cd.textContent).toBe('in 1');
      expect(ended).toBe(0);
      vi.advanceTimersByTime(1000);
      expect(ended).toBe(1);
      expect(cd.textContent).toBe('ready');
      expect(utils.countdownActive(cd)).toBe(false);
    } finally {
      vi.useRealTimers();
    }
  });

  it('a second start supersedes the first — one writer per node', () => {
    vi.useFakeTimers();
    try {
      const cd = document.createElement('span');
      utils.startCountdown(cd, 5, (s) => `A${s}`, () => { cd.textContent = 'A-done'; });
      utils.startCountdown(cd, 2, (s) => `B${s}`, () => { cd.textContent = 'B-done'; });
      expect(utils.countdownActive(cd)).toBe(true);
      vi.advanceTimersByTime(2000);
      // Only B's timer exists: A's stale timer must not rewrite the label.
      expect(cd.textContent).toBe('B-done');
      vi.advanceTimersByTime(1000);
      expect(cd.textContent).toBe('B-done');
    } finally {
      vi.useRealTimers();
    }
  });

  it('stopCountdown halts the writer and clears active state', () => {
    vi.useFakeTimers();
    try {
      const cd = document.createElement('span');
      utils.startCountdown(cd, 5, (s) => `${s}`, () => {});
      utils.stopCountdown(cd);
      expect(utils.countdownActive(cd)).toBe(false);
      const frozen = cd.textContent;
      vi.advanceTimersByTime(3000);
      expect(cd.textContent).toBe(frozen);
    } finally {
      vi.useRealTimers();
    }
  });

  it('tolerates a missing node', () => {
    expect(() => utils.startCountdown(null, 5, (s) => `${s}`, () => {})).not.toThrow();
    expect(() => utils.stopCountdown(null)).not.toThrow();
    expect(utils.countdownActive(null)).toBe(false);
  });
});

describe('admin-utils fxTimeLabel (FX chip shows WIB, not raw UTC)', () => {
  it('shifts a UTC ISO timestamp to UTC+7', () => {
    expect(utils.fxTimeLabel('2026-06-01T12:51:00Z')).toBe('19:51 UTC+7');
    // Date-line rollover: 23:00 UTC is 06:00 the NEXT day in WIB.
    expect(utils.fxTimeLabel('2026-06-01T23:00:00Z')).toBe('06:00 UTC+7');
    expect(utils.fxTimeLabel('2026-06-01T16:59:59Z')).toBe('23:59 UTC+7');
  });

  it('accepts an explicit +HH:MM offset', () => {
    // 09:00 at +00:30 = 08:30 UTC = 15:30 UTC+7.
    expect(utils.fxTimeLabel('2026-06-01T09:00:00+00:30')).toBe('15:30 UTC+7');
  });

  it('falls back to raw UTC for a zone-less timestamp (no double-shift)', () => {
    expect(utils.fxTimeLabel('2026-06-01T12:51:00')).toBe('12:51 UTC');
  });

  it('returns empty for junk or missing input so the suffix is dropped', () => {
    expect(utils.fxTimeLabel('')).toBe('');
    expect(utils.fxTimeLabel(undefined)).toBe('');
    expect(utils.fxTimeLabel('not-a-date')).toBe('');
  });
});

describe('admin-utils logView / stripAnsi / logTsWib (health platform logs)', () => {
  it('stripAnsi removes color, cursor and OSC escapes', () => {
    expect(utils.stripAnsi('\x1b[32mok\x1b[0m')).toBe('ok');
    expect(utils.stripAnsi('\x1b[2Jclear')).toBe('clear');
    expect(utils.stripAnsi('\x1b]0;title\x07tail')).toBe('tail');
    expect(utils.stripAnsi(null)).toBe('');
  });

  it('logTsWib shifts UTC to WIB with seconds', () => {
    expect(utils.logTsWib('2026-09-01T12:51:17.310Z')).toBe('19:51:17');
    expect(utils.logTsWib('garbage')).toBe('');
  });

  it('logView renders rows via textContent (no markup injection)', () => {
    const v = utils.logView([{ ts: '2026-09-01T12:51:17.310Z', log: '<img src=x onerror=alert(1)>' }]);
    expect(v.querySelectorAll('.log-line').length).toBe(1);
    expect(v.querySelector('.log-msg').textContent).toBe('<img src=x onerror=alert(1)>');
    expect(v.querySelector('.log-msg').querySelector('img')).toBeNull();
    expect(v.querySelector('.log-ts').textContent).toBe('19:51:17');
  });

  it('logView shows the empty state for no lines', () => {
    expect(utils.logView([]).querySelector('.empty')).not.toBeNull();
    expect(utils.logView(null).querySelector('.empty')).not.toBeNull();
  });
});

describe('admin-utils cfDeployRows (health Cloudflare deployments)', () => {
  const deploys = [
    { id: 'd1', time: '2026-09-01T14:12:07.544Z', author: 'adikaradwiatmaja@gmail.com', trigger: 'deployment', message: 'Coding Agent — 47e3ea90 health logs (feat/agent-4-website)', versionId: 'v1' },
    { id: 'd2', time: '2026-09-01T14:08:43.612Z', author: 'adikaradwiatmaja@gmail.com', trigger: 'secret', message: '', versionId: 'v2' },
  ];

  it('renders rows newest-first with sha highlighted via textContent', () => {
    const v = utils.cfDeployRows(deploys);
    const rows = v.querySelectorAll('.deploy-row');
    expect(rows.length).toBe(2);
    const sha = rows[0].querySelector('.deploy-sha');
    expect(sha.textContent).toBe('47e3ea90');
    expect(rows[0].querySelector('.deploy-msg').querySelector('img')).toBeNull();
    expect(rows[0].innerHTML).not.toContain('<img');
    expect(rows[0].querySelector('.deploy-time').textContent).toBe('21:12:07 WIB');
  });

  it('tags secret-triggered rows with the warning chip and falls back to —', () => {
    const v = utils.cfDeployRows(deploys);
    const rows = v.querySelectorAll('.deploy-row');
    expect(rows[1].querySelector('.deploy-chip--secret')).not.toBeNull();
    expect(rows[1].querySelector('.deploy-msg').textContent).toBe('—');
  });

  it('shows the empty state for no deploys', () => {
    expect(utils.cfDeployRows([]).querySelector('.empty')).not.toBeNull();
    expect(utils.cfDeployRows(null).querySelector('.empty')).not.toBeNull();
  });
});

describe('admin-utils relTime / sparkline / nfStatusCard / uptimeRows (health v2)', () => {
  const NOW = Date.parse('2026-09-01T15:00:00Z');

  it('relTime buckets deltas and handles junk', () => {
    expect(utils.relTime('2026-09-01T14:59:56Z', NOW)).toBe('just now');
    expect(utils.relTime('2026-09-01T14:59:30Z', NOW)).toBe('30s ago');
    expect(utils.relTime('2026-09-01T14:56:00Z', NOW)).toBe('4m ago');
    expect(utils.relTime('2026-09-01T12:00:00Z', NOW)).toBe('3h ago');
    expect(utils.relTime('2026-08-30T15:00:00Z', NOW)).toBe('2d ago');
    expect(utils.relTime('garbage', NOW)).toBe('');
  });

  it('sparkline draws an area path starting with M and WIB edge labels', () => {
    const svg = utils.sparkline([
      { t: '2026-09-01T13:00:00Z', req: 4, err: 0 },
      { t: '2026-09-01T13:01:00Z', req: 9, err: 1 },
      { t: '2026-09-01T13:02:00Z', req: 2, err: 0 },
    ]);
    expect(svg).toContain('class="chart-svg spark-svg"');
    expect(svg).toMatch(/d="M \d/);
    expect(svg).toContain('stroke="var(--danger)"');
    expect(svg).toContain('20:00');
    expect(svg).toContain('20:02');
  });

  it('sparkline falls back to the empty state', () => {
    expect(utils.sparkline([])).toContain('chart-empty');
    expect(utils.sparkline([{ t: 'x', req: 0 }])).toContain('chart-empty');
  });

  it('nfStatusCard renders status chip, running sha and fields', () => {
    const v = utils.nfStatusCard({
      deploymentStatus: 'COMPLETED', deploymentReason: 'DEPLOYING', buildStatus: 'SUCCESS',
      deployedSha: 'e0046a6fbd507b02f60bd1a868c53f83dea167bf', branch: 'main',
      region: 'nf-europe-west', instances: 1, updatedAt: '2026-09-01T03:20:20.172Z',
    });
    expect(v.querySelector('.status-ok')).not.toBeNull();
    expect(v.querySelector('.deploy-sha').textContent).toBe('e0046a6f');
    expect(v.textContent).toContain('main');
    expect(v.textContent).toContain('nf-europe-west');
  });

  it('nfStatusCard marks non-completed deployment as warn', () => {
    const v = utils.nfStatusCard({ deploymentStatus: 'DEPLOYING', instances: 2 });
    expect(v.querySelector('.status-warn')).not.toBeNull();
  });

  it('uptimeRows renders dots, latency and error text', () => {
    const v = utils.uptimeRows([
      { name: 'license api', up: true, ms: 42, vantage: 'edge' },
      { name: 'admin', up: false, ms: 5000, error: 'HTTP 502' },
    ]);
    const rows = v.querySelectorAll('.up-row');
    expect(rows.length).toBe(2);
    expect(rows[0].querySelector('.up-dot--ok')).not.toBeNull();
    expect(rows[0].querySelector('.up-ms').textContent).toBe('42 ms');
    expect(rows[0].querySelector('.up-vantage').textContent).toBe('edge');
    expect(rows[1].querySelector('.up-dot--bad')).not.toBeNull();
    expect(rows[1].querySelector('.up-err').textContent).toBe('HTTP 502');
    expect(rows[1].querySelector('.up-vantage')).toBeNull();
  });

  it('logView highlights error lines', () => {
    const v = utils.logView([
      { ts: '2026-09-01T12:00:00Z', log: 'all good' },
      { ts: '2026-09-01T12:01:00Z', log: 'scanner: failed to connect' },
    ]);
    const rows = v.querySelectorAll('.log-line');
    expect(rows[0].className).not.toContain('log-line--err');
    expect(rows[1].className).toContain('log-line--err');
  });
});

describe('admin-utils phone chart variants (mobile 1:1 canvases)', () => {
  const months = Array.from({ length: 8 }, (_, i) => ({ month: '2026-0' + (i + 1), idr: 1000000 + i * 500000, count: 5 + i, churn: i % 3 }));

  it('svgChart phone canvas is ~350 wide with larger labels and sparse ticks', () => {
    const svg = utils.svgChart('rev', months, ['idr'], { area: true, phone: true, fmt: (v: number) => 'Rp' + (v / 1000000).toFixed(1) + 'jt' });
    expect(svg).toContain('viewBox="0 0 350 230"');
    expect(svg).toContain('font-size="11"');
    expect(svg).toContain('font-size="10"');
    // step 3 on 8 points → labels at 0,3,6,7 (last always shown)
    expect(svg.match(/text-anchor="middle" fill="var\(--muted\)"/g)!.length).toBe(4);
    expect(svg).toMatch(/d="M \d/);
  });

  it('svgChart wide canvas is 1280×230 with readable font sizes', () => {
    const svg = utils.svgChart('rev', months, ['idr'], { wide: true });
    expect(svg).toContain('viewBox="0 0 1280 230"');
    expect(svg).toContain('font-size="13"');
    expect(svg).toContain('font-size="12"');
  });

  it('svgBarChart phone canvas is ~350×200 with 11px values', () => {
    const svg = utils.svgBarChart('s', months, { valueKey: 'count', phone: true });
    expect(svg).toContain('viewBox="0 0 350 200"');
    expect(svg).toContain('font-size="11" font-weight="600"');
    expect(svg).not.toContain('max-height');
  });

  it('sparkline renders gridlines + HTML y-labels (readable values)', () => {
    const svg = utils.sparkline([
      { t: '2026-09-01T13:00:00Z', req: 4, err: 0 },
      { t: '2026-09-01T13:01:00Z', req: 900, err: 1 },
    ]);
    // 5 gridlines: dashed above, solid baseline
    expect(svg.match(/<line /g)!.length).toBe(5);
    expect(svg.match(/stroke-dasharray="4 4"/g)!.length).toBe(4);
    // y-label column is HTML (5 labels), values compact (900 stays, top = max)
    expect(svg).toContain('class="spark-y"');
    expect(svg.match(/<span style="top:/g)!.length).toBe(5);
    expect(svg).toContain('>900</span>');
    expect(svg).toContain('>0</span>');
    // no SVG <text> — labels must not stretch with preserveAspectRatio=none
    expect(svg).not.toContain('<text');
    // time labels moved to the HTML bottom row (WIB)
    expect(svg).toContain('class="spark-x"');
    expect(svg).toContain('20:00');
  });
});

describe('admin-utils busyWrap (B19: double-click submitted the action twice)', () => {
  // The tenant detail modal's Revoke/Activate/Renew/Upgrade-save buttons
  // fired doAction on every click with no in-flight guard. Renew is
  // +365 days per POST — a double-click granted 730 days.
  it('ignores re-entry while the first call is in flight, re-enables after', async () => {
    let calls = 0;
    let release: () => void = () => {};
    const btn = document.createElement('button');
    const slow = utils.busyWrap(btn, () => {
      calls++;
      return new Promise<void>((r) => { release = r; });
    });
    const p1 = slow();
    slow();
    slow();
    expect(calls).toBe(1);
    expect(btn.disabled).toBe(true);
    release();
    await p1;
    expect(btn.disabled).toBe(false);
    const p2 = slow();
    release();
    await p2;
    expect(calls).toBe(2);
  });

  it('re-enables even when the handler rejects', async () => {
    const btn = document.createElement('button');
    const boom = utils.busyWrap(btn, () => Promise.reject(new Error('x')));
    await expect(boom()).rejects.toThrow('x');
    expect(btn.disabled).toBe(false);
  });

  it('passes through sync handlers and tolerates a null button', () => {
    let calls = 0;
    const fn = utils.busyWrap(null, () => { calls++; });
    fn();
    fn();
    expect(calls).toBe(2);
  });
});

describe('admin-utils AbortSignal.timeout availability (B20: regression from B10/B12)', () => {
  // B10/B12 attached AbortSignal.timeout unconditionally. That static is
  // Chrome/WebView 103+ and Safari 16+ — on an older Android WebView the
  // call throws TypeError, which in fetchWithTimeout rejected EVERY api()
  // call: the whole dashboard broke on browsers that worked before.
  // Without the primitive the fetch must proceed un-timed (old behavior)
  // rather than throw.
  const withoutTimeout = async (fn: () => Promise<void>) => {
    const desc = Object.getOwnPropertyDescriptor(AbortSignal, 'timeout');
    if (!desc) throw new Error('AbortSignal.timeout missing entirely — nothing to remove');
    Object.defineProperty(AbortSignal, 'timeout', { value: undefined, configurable: true, writable: true });
    try {
      await fn();
    } finally {
      Object.defineProperty(AbortSignal, 'timeout', desc);
    }
  };

  it('fetchWithTimeout still performs the fetch when AbortSignal.timeout is absent', async () => {
    await withoutTimeout(async () => {
      let seen: any = null;
      const fetchImpl = async (_u: string, o?: any) => { seen = o; return { ok: true }; };
      const res: any = await utils.fetchWithTimeout(fetchImpl, 'https://x/api', {}, 5000);
      expect(res.ok).toBe(true);
      expect(seen.signal).toBeUndefined(); // no signal — not a throw
    });
  });

  it('fetchFxRate still resolves a live rate when AbortSignal.timeout is absent', async () => {
    await withoutTimeout(async () => {
      const fetchImpl = async () => ({ json: async () => ({ rates: { IDR: 16000 } }) });
      const r = await utils.fetchFxRate(fetchImpl, 5000);
      expect(r.live).toBe(true);
      expect(r.rate).toBe(16000);
    });
  });

  it('restores the primitive afterwards (no test pollution)', () => {
    expect(typeof AbortSignal.timeout).toBe('function');
  });
});

describe('admin-utils setAuthMode (B21: tab switch mid-submit corrupts the other mode)', () => {
  // login.js tab buttons call setAuthMode unconditionally. Clicking
  // Password while an OTP request is in flight flips currentMode; when
  // the response lands, the completion path writes the OTP-mode label
  // onto the password tab's button ("Send Verification Code" with a
  // password field showing) and can even start the OTP cooldown while
  // the user is on the password tab. setAuthMode must refuse mode flips
  // while a submit is in flight.
  const buildLoginDom = () => {
    document.body.innerHTML = `
      <button id="tab-otp"></button><button id="tab-password"></button>
      <div id="password-group"></div><div id="otp-group" class="hidden"></div>
      <button id="login-btn"></button><div id="otp-cooldown" class="hidden"></div>`;
    return {
      tabOtp: document.getElementById('tab-otp')!,
      tabPwd: document.getElementById('tab-password')!,
      pwdGroup: document.getElementById('password-group')!,
      otpGroup: document.getElementById('otp-group')!,
      loginBtn: document.getElementById('login-btn')!,
      cd: document.getElementById('otp-cooldown')!,
    };
  };

  it('switches modes normally when not submitting', () => {
    const d = buildLoginDom();
    utils.setAuthMode('password', d, { isSubmitting: () => false });
    expect(d.pwdGroup.classList.contains('hidden')).toBe(false);
    expect(d.tabPwd.classList.contains('active')).toBe(true);
    expect(d.loginBtn.textContent).toBe('Sign In with Password');
  });

  it('refuses the flip while a submit is in flight', () => {
    const d = buildLoginDom();
    utils.setAuthMode('otp', d, { isSubmitting: () => false }); // baseline
    const pwdLabel = 'Sign In with Password';
    utils.setAuthMode('password', d, { isSubmitting: () => true });
    // Nothing changed: still OTP mode.
    expect(d.tabOtp.classList.contains('active')).toBe(true);
    expect(d.pwdGroup.classList.contains('hidden')).toBe(true);
    expect(d.loginBtn.textContent).not.toBe(pwdLabel);
  });

  it('still re-shows an active cooldown when not submitting', () => {
    vi.useFakeTimers();
    try {
      const d = buildLoginDom();
      utils.setAuthMode('otp', d, { isSubmitting: () => false });
      utils.startCountdown(d.cd, 30, (s) => `in ${s}`, () => {});
      d.cd.classList.add('hidden'); // pretend a password switch hid it
      utils.setAuthMode('otp', d, { isSubmitting: () => false });
      expect(d.cd.classList.contains('hidden')).toBe(false);
    } finally {
      vi.useRealTimers();
    }
  });
});

describe('admin-utils mountModal focus (B27: dialog announced but focus never entered)', () => {
  function buildModalDom() {
    document.body.innerHTML =
      '<div id="modal-root"></div><button id="trigger">open</button>';
    return {
      root: document.getElementById('modal-root')!,
      trigger: document.getElementById('trigger') as HTMLButtonElement,
    };
  }

  it('moves focus into the dialog when mounted', () => {
    const d = buildModalDom();
    d.trigger.focus();
    const box = document.createElement('div');
    box.setAttribute('role', 'dialog');
    box.innerHTML = '<button id="ok">OK</button>';
    utils.mountModal(d.root, box);
    // Focus must be inside the dialog — not left on the trigger behind
    // the backdrop (keyboard/SR users tabbed through hidden content).
    expect(d.root.contains(document.activeElement)).toBe(true);
  });

  it('restores focus to the opener when closed', () => {
    const d = buildModalDom();
    d.trigger.focus();
    const box = document.createElement('div');
    box.innerHTML = '<button id="ok">OK</button>';
    const close = utils.mountModal(d.root, box);
    close();
    expect(document.activeElement).toBe(d.trigger);
  });

  it('focuses the dialog box itself when it has no focusable child yet', () => {
    // The tenant-detail modal mounts with "Loading…" — no buttons until
    // the fetch resolves. The box gets tabindex=-1 and takes focus.
    const d = buildModalDom();
    d.trigger.focus();
    const box = document.createElement('div');
    box.textContent = 'Loading…';
    utils.mountModal(d.root, box);
    expect(document.activeElement).toBe(box);
    expect(box.getAttribute('tabindex')).toBe('-1');
  });

  it('survives close when the opener was removed (re-rendered table)', () => {
    const d = buildModalDom();
    d.trigger.focus();
    const box = document.createElement('div');
    box.innerHTML = '<button id="ok">OK</button>';
    const close = utils.mountModal(d.root, box);
    d.trigger.remove(); // e.g. renderTenants replaced the table
    expect(() => close()).not.toThrow();
  });

  // B28: B27 moved focus INTO the dialog, but Tab still walked out of it
  // into the background page behind the backdrop (WCAG 2.1.2 — no
  // keyboard trap in, no escape out). mountModal must cycle Tab within
  // the dialog's focusables while it is open.
  it('Tab on the last focusable wraps to the first', () => {
    const d = buildModalDom();
    const box = document.createElement('div');
    box.innerHTML = '<button id="a">A</button><button id="b">B</button>';
    utils.mountModal(d.root, box);
    const b = document.getElementById('b')!;
    b.focus();
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Tab', bubbles: true }));
    expect(document.activeElement!.id).toBe('a');
  });

  it('Shift+Tab on the first focusable wraps to the last', () => {
    const d = buildModalDom();
    const box = document.createElement('div');
    box.innerHTML = '<button id="a">A</button><button id="b">B</button>';
    utils.mountModal(d.root, box);
    const a = document.getElementById('a')!;
    a.focus();
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Tab', shiftKey: true, bubbles: true }));
    expect(document.activeElement!.id).toBe('b');
  });

  it('Shift+Tab from the box itself (loading state) wraps to the last', () => {
    const d = buildModalDom();
    const box = document.createElement('div');
    box.textContent = 'Loading…';
    utils.mountModal(d.root, box);
    // Content arrives later — focus is on the box; after buttons render,
    // Shift+Tab from the box must land on the LAST focusable, not escape.
    box.innerHTML = '<button id="a">A</button><button id="b">B</button>';
    box.focus();
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Tab', shiftKey: true, bubbles: true }));
    expect(document.activeElement!.id).toBe('b');
  });

  it('Tab from a background element pulls focus back into the dialog', () => {
    const d = buildModalDom();
    const box = document.createElement('div');
    box.innerHTML = '<button id="a">A</button>';
    utils.mountModal(d.root, box);
    d.trigger.focus(); // user clicked background content (or SR moved there)
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Tab', bubbles: true }));
    expect(box.contains(document.activeElement)).toBe(true);
  });

  it('the trap is gone after close (background Tab is natural again)', () => {
    const d = buildModalDom();
    const box = document.createElement('div');
    box.innerHTML = '<button id="a">A</button>';
    const close = utils.mountModal(d.root, box);
    close();
    d.trigger.focus();
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Tab', bubbles: true }));
    // Handler removed: focus must NOT be yanked into the (unmounted) box.
    expect(document.activeElement).toBe(d.trigger);
  });
});

describe('admin-utils property fuzz — normalizeStats + renderers under hostile payloads (r8)', () => {
  // Deterministic LCG: a failure reproduces exactly (seed pinned).
  function lcg(seed: number) {
    let s = seed >>> 0;
    return () => ((s = (s * 1664525 + 1013904223) >>> 0) / 0x100000000);
  }
  const rnd = lcg(0x20260830);
  const weird: any[] = [
    null, undefined, 0, -1, NaN, Infinity, -Infinity, '', 'abc', '123',
    true, false, {}, [], [1, 2], { a: 1 }, () => 0, new Date(), 1e308,
  ];
  const pick = () => weird[Math.floor(rnd() * weird.length)];
  const ARRAY_KEYS = [
    'revenueTrend', 'subscriberGrowth', 'tierDistribution', 'providerSplit',
    'signupsPerMonth', 'churnPerMonth', 'topSubscribers', 'recentSignups',
    'expiringSoon',
  ] as const;

  function fuzzPayload(): any {
    const raw: any = {};
    for (const key of ARRAY_KEYS) {
      const r = rnd();
      if (r < 0.2) raw[key] = pick();
      else if (r < 0.45) raw[key] = Array.from({ length: Math.floor(rnd() * 5) }, pick);
    }
    const k: any = {};
    for (const kk of ['mrrUsd', 'mrrIdr', 'lifetimeUsd', 'lifetimeIdr',
      'totalUsers', 'arpuUsd', 'fxRate', 'activeUsers', 'totalSubscribers',
      'activeDevices', 'trialToPaidRate']) {
      if (rnd() < 0.6) k[kk] = pick();
    }
    if (rnd() < 0.2) raw.kpis = pick(); else raw.kpis = k;
    return raw;
  }

  it('normalizeStats never throws and always restores the shape contract', () => {
    // The contract: the NUMERIC KPI keys (the server's money/count set)
    // must come back finite — a NaN slipping through renders "$NaN"/
    // "Rp NaN" text in the cards (the B4 class). Non-numeric keys
    // (fxUpdatedAt string, fxLive boolean) pass through untouched.
    const NUMERIC_KPIS = ['totalUsers', 'activeUsers', 'totalSubscribers', 'activeDevices',
      'mrrUsd', 'mrrIdr', 'lifetimeUsd', 'lifetimeIdr', 'arpuUsd', 'trialToPaidRate', 'fxRate'];
    for (let i = 0; i < 300; i++) {
      let m: any;
      expect(() => { m = utils.normalizeStats(fuzzPayload()); }).not.toThrow();
      for (const key of ARRAY_KEYS) expect(Array.isArray(m[key])).toBe(true);
      expect(m.kpis && typeof m.kpis === 'object').toBe(true);
      for (const key of NUMERIC_KPIS) {
        const v = m.kpis[key];
        expect(typeof v === 'number' && Number.isFinite(v)).toBe(true);
      }
    }
  });

  it('chart builders tolerate fuzzed rows without throwing', () => {
    for (let i = 0; i < 100; i++) {
      const m = utils.normalizeStats(fuzzPayload());
      expect(() => utils.svgChart('a', m.revenueTrend, ['usd', 'idr'], { area: true })).not.toThrow();
      expect(() => utils.svgBarChart('b', m.signupsPerMonth, { valueKey: 'count' })).not.toThrow();
      expect(() => utils.svgDonut('d', m.tierDistribution, 'tier', 'count', ['#fff'])).not.toThrow();
    }
  });

  it('tenant row builders tolerate fuzzed tenants', () => {
    for (let i = 0; i < 100; i++) {
      const t: any = {};
      for (const key of ['id', 'email', 'status', 'emailVerified', 'created', 'license', 'subscription']) {
        if (rnd() < 0.7) t[key] = pick();
      }
      expect(() => utils.tenantRow(t, () => {})).not.toThrow();
      expect(() => utils.tenantDetailRows(fuzzPayload())).not.toThrow();
    }
  });
});

describe('admin-utils countdown seconds coercion (B39: NaN seconds = permanent lockout)', () => {
  // login.js passes body.retry_after from the server. A stringy/garbage
  // value ("abc", NaN) made `remaining -= 1` produce NaN forever —
  // NaN <= 0 is false, so the interval NEVER ended: the login button
  // stayed disabled until a full page reload.
  it('lockout with non-numeric seconds ends immediately (button restored)', () => {
    vi.useFakeTimers();
    try {
      const btn = document.createElement('button');
      btn.textContent = 'Sign In';
      utils.startLockoutCountdown(btn, 'abc' as any, (s) => `in ${s}`, () => 'Sign In');
      // Not left disabled, no live timer, label restored via the end path.
      expect(btn.disabled).toBe(false);
      expect(utils.isLockoutActive(btn)).toBe(false);
      expect(btn.textContent).toBe('Sign In');
    } finally {
      vi.useRealTimers();
    }
  });

  it('numeric-string seconds still run a real countdown', () => {
    vi.useFakeTimers();
    try {
      const btn = document.createElement('button');
      utils.startLockoutCountdown(btn, '3' as any, (s) => `in ${s}`, () => 'Go');
      expect(btn.disabled).toBe(true);
      expect(btn.textContent).toBe('in 3');
      vi.advanceTimersByTime(3000);
      expect(btn.disabled).toBe(false);
      expect(btn.textContent).toBe('Go');
    } finally {
      vi.useRealTimers();
    }
  });

  it('cooldown with NaN seconds fires onEnd instead of spinning forever', () => {
    vi.useFakeTimers();
    try {
      const node = document.createElement('div');
      let ended = 0;
      utils.startCountdown(node, NaN, (s) => `in ${s}`, () => { ended++; });
      expect(ended).toBe(1);
      expect(utils.countdownActive(node)).toBe(false);
      vi.advanceTimersByTime(5000);
      expect(ended).toBe(1); // no zombie interval
    } finally {
      vi.useRealTimers();
    }
  });
});

describe('admin-utils setAuthMode aria-selected sync (B40: stale tab state for AT)', () => {
  function buildLoginDom() {
    document.body.innerHTML = `
      <div role="tablist">
        <button id="tab-otp" role="tab" aria-selected="true">OTP</button>
        <button id="tab-password" role="tab" aria-selected="false">Pwd</button>
      </div>
      <div id="password-group"></div><div id="otp-group" class="hidden"></div>
      <button id="login-btn"></button><div id="otp-cooldown" class="hidden"></div>`;
    return {
      tabOtp: document.getElementById('tab-otp')!,
      tabPwd: document.getElementById('tab-password')!,
      pwdGroup: document.getElementById('password-group')!,
      otpGroup: document.getElementById('otp-group')!,
      loginBtn: document.getElementById('login-btn')!,
      cd: document.getElementById('otp-cooldown')!,
    };
  }

  it('flipping to password moves aria-selected with the visual state', () => {
    const d = buildLoginDom();
    utils.setAuthMode('password', d, { isSubmitting: () => false });
    expect(d.tabPwd.getAttribute('aria-selected')).toBe('true');
    expect(d.tabOtp.getAttribute('aria-selected')).toBe('false');
  });

  it('flipping back to otp restores aria-selected', () => {
    const d = buildLoginDom();
    utils.setAuthMode('password', d, { isSubmitting: () => false });
    utils.setAuthMode('otp', d, { isSubmitting: () => false });
    expect(d.tabOtp.getAttribute('aria-selected')).toBe('true');
    expect(d.tabPwd.getAttribute('aria-selected')).toBe('false');
  });
});

describe('admin-utils tableCard malformed input (B41: B36 class in the table builder)', () => {
  it('null rows and non-array headers do not crash the builder', () => {
    expect(() => {
      utils.tableCard('T', ['A', 'B'], [null, ['ok'], 42, undefined]);
    }).not.toThrow();
    // Malformed rows are skipped; valid ones still render.
    const card = utils.tableCard('T', ['A'], [['x']]);
    expect(card.querySelectorAll('tbody tr').length).toBe(1);
    // Non-array rows entirely → empty-state, no throw.
    expect(() => utils.tableCard('T', ['A'], { not: 'rows' } as any)).not.toThrow();
    expect(() => utils.tableCard('T', null as any, [['a']])).not.toThrow();
  });
});

describe('admin-utils chart tooltips (#9: chartTipText / nearestChartIndex / bindChartTooltip)', () => {
  it('chartTipText shows the month and each series value', () => {
    const text = utils.chartTipText({ month: '2026-09', idr: 68000000 }, [{ key: 'idr', label: 'Gross' }], (v: number) => 'Rp' + (v / 1000000).toFixed(1) + 'jt');
    expect(text).toContain('2026');
    expect(text).toContain('Gross: Rp68.0jt');
  });

  it('chartTipText skips non-finite series values', () => {
    const text = utils.chartTipText({ month: '2026-01', a: 5, b: 'nope' }, [{ key: 'a', label: 'A' }, { key: 'b', label: 'B' }]);
    expect(text).toContain('A: 5');
    expect(text).not.toContain('B');
  });

  it('chartTipText handles missing row/month safely', () => {
    expect(utils.chartTipText(null, [{ key: 'a', label: 'A' }])).toBe('');
    expect(utils.chartTipText({}, [{ key: 'a', label: 'A' }])).toBe('');
  });

  it('nearestChartIndex clamps to valid indices', () => {
    expect(utils.nearestChartIndex(0, 12)).toBe(0);
    expect(utils.nearestChartIndex(1, 12)).toBe(11);
    expect(utils.nearestChartIndex(-5, 12)).toBe(0);
    expect(utils.nearestChartIndex(99, 12)).toBe(11);
    expect(utils.nearestChartIndex(0.5, 2)).toBe(1);
    expect(utils.nearestChartIndex(0.4, 2)).toBe(0);
    expect(utils.nearestChartIndex(0.9, 1)).toBe(0);
  });

  it('bindChartTooltip attaches a hidden tip and shows it on mousemove', () => {
    const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
    svg.setAttribute('class', 'chart-svg');
    const card = document.createElement('div');
    card.className = 'chart-card';
    card.appendChild(svg);
    document.body.appendChild(card);
    // jsdom returns a zero rect; the binder treats width===0 as "no chart
    // yet", so stub a real-ish rect to exercise the show path.
    svg.getBoundingClientRect = () => ({ left: 0, top: 0, width: 200, height: 100, right: 200, bottom: 100, x: 0, y: 0 } as DOMRect);
    try {
      utils.bindChartTooltip(svg, [{ month: '2026-01', count: 3 }, { month: '2026-02', count: 5 }], [{ key: 'count', label: 'N' }]);
      const tip = card.querySelector('.chart-tip');
      expect(tip).not.toBeNull();
      expect((tip as HTMLElement).style.display).toBe('none');
      // Hover at x=100 of 200 → ratio 0.5 → index 1 ('N: 5').
      svg.dispatchEvent(new MouseEvent('mousemove', { clientX: 100, clientY: 10 }));
      expect((tip as HTMLElement).textContent).toContain('N: 5');
      expect((tip as HTMLElement).style.display).toBe('block');
      svg.dispatchEvent(new MouseEvent('mouseleave'));
      expect((tip as HTMLElement).style.display).toBe('none');
    } finally {
      document.body.removeChild(card);
    }
  });

  // Regression (#10 tooltip geometry): on a 12-month bar chart the last bar
  // (current month) sits at x=585 of a 620-unit viewBox (slots start at x=10,
  // width (620-20)/12=50). The old linear-ratio mapping snapped that cursor
  // position to index 10 — the PREVIOUS month. The slot-based mapping must
  // return index 11 for the current month, and index 0 at the first bar.
  it('bar-chart tooltip maps the current-month slot to the last index (not the previous month)', () => {
    const months = Array.from({ length: 12 }, (_, i) => `2026-${String(i + 1).padStart(2, '0')}`);
    const rows = months.map((month, i) => ({ month, count: i + 1 }));
    const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
    svg.setAttribute('class', 'chart-svg');
    svg.setAttribute('viewBox', '0 0 620 200');
    const card = document.createElement('div');
    card.className = 'chart-card';
    card.appendChild(svg);
    document.body.appendChild(card);
    svg.getBoundingClientRect = () => ({ left: 0, top: 0, width: 620, height: 200, right: 620, bottom: 200, x: 0, y: 0 } as DOMRect);
    try {
      utils.bindChartTooltip(svg, rows, [{ key: 'count', label: 'N' }]);
      const tip = card.querySelector('.chart-tip') as HTMLElement;
      // Last bar center: 10 + 11.5*50 = 585 → current month (index 11).
      svg.dispatchEvent(new MouseEvent('mousemove', { clientX: 585, clientY: 10 }));
      expect(tip.textContent).toContain('Dec 2026');
      // First bar center: 10 + 0.5*50 = 35 → January (index 0).
      svg.dispatchEvent(new MouseEvent('mousemove', { clientX: 35, clientY: 10 }));
      expect(tip.textContent).toContain('Jan 2026');
      // Mid-slot boundary test: x=410 is inside slot 8 (10+8*50=410) → September.
      svg.dispatchEvent(new MouseEvent('mousemove', { clientX: 410, clientY: 10 }));
      expect(tip.textContent).toContain('Sep 2026');
    } finally {
      document.body.removeChild(card);
    }
  });

  // Line-chart geometry: points sit at px + (i/(n-1))*pw with px=40, pw=560
  // for the narrow 600-unit canvas. The old linear ratio also skewed line
  // charts; the kind:'line' branch maps by point position instead.
  it('line-chart tooltip snaps to the nearest point by viewBox geometry', () => {
    const rows = [
      { month: '2026-01', idr: 10 },
      { month: '2026-02', idr: 20 },
      { month: '2026-03', idr: 30 },
      { month: '2026-04', idr: 40 },
    ];
    const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
    svg.setAttribute('class', 'chart-svg');
    svg.setAttribute('viewBox', '0 0 600 200');
    const card = document.createElement('div');
    card.className = 'chart-card';
    card.appendChild(svg);
    document.body.appendChild(card);
    svg.getBoundingClientRect = () => ({ left: 0, top: 0, width: 600, height: 200, right: 600, bottom: 200, x: 0, y: 0 } as DOMRect);
    try {
      utils.bindChartTooltip(svg, rows, [{ key: 'idr', label: 'Gross' }], undefined, 'line');
      const tip = card.querySelector('.chart-tip') as HTMLElement;
      // Last point: px + 3*(560/3) = 600 → April (index 3).
      svg.dispatchEvent(new MouseEvent('mousemove', { clientX: 600, clientY: 10 }));
      expect(tip.textContent).toContain('Apr 2026');
      // First point: x=40 → January.
      svg.dispatchEvent(new MouseEvent('mousemove', { clientX: 40, clientY: 10 }));
      expect(tip.textContent).toContain('Jan 2026');
      // Mid-chart: x=340 → between points 1 (226) and 2 (413), closer to 2.
      svg.dispatchEvent(new MouseEvent('mousemove', { clientX: 340, clientY: 10 }));
      expect(tip.textContent).toContain('Mar 2026');
    } finally {
      document.body.removeChild(card);
    }
  });
});
