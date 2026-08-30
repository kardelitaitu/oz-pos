// @vitest-environment jsdom
// Unit tests for the admin dashboard's pure helpers (H2 hardening).
// The helpers live in public/admin/admin-utils.js — a UMD module that
// exports for Node/vitest and defines window.AdminUtils in the browser.
import { describe, expect, it, vi } from 'vitest';
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

describe('admin-utils tenantDetailRows (B2: t() shadowing regression)', () => {
  const data = {
    tenant: { status: 'active', emailVerified: true, created: '2026-08-01T10:00:00Z' },
    license: { key: 'OZ-KEY' },
    subscription: { tierKey: 'pro', status: 'active', expiresAt: '2027-08-01' },
    devices: [{ id: 'd1' }, { id: 'd2' }],
  };

  it('builds the 8 key/value rows without crashing', () => {
    const rows = utils.tenantDetailRows(data);
    expect(rows.length).toBe(8);
    // B16 superseded the raw-enum expectation: status rows carry labels.
    expect(rows[0]).toEqual(['Status', 'Active']);
    expect(rows[1]).toEqual(['Email verified', '✓']);
    expect(rows[2]).toEqual(['Created', '2026-08-01']);
    expect(rows[3]).toEqual(['License key', 'OZ-KEY']);
    expect(rows[4]).toEqual(['Tier', 'pro']);
    expect(rows[5]).toEqual(['Subscription status', 'Active']);
    expect(rows[6]).toEqual(['Expires', '2027-08-01']);
    expect(rows[7]).toEqual(['Devices', 2]);
  });

  it('handles a fully empty payload with em-dash values', () => {
    const rows = utils.tenantDetailRows({});
    expect(rows.length).toBe(8);
    expect(rows[0][1]).toBe('—');
    expect(rows[7][1]).toBe(0);
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
    // max=10 → full height 140; 5 → half height 70.
    expect(svg).toContain('height="140"');
    expect(svg).toContain('height="70"');
    expect(svg).not.toContain('NaN');
  });

  it('defaults to count for signup-shaped data', () => {
    const svg = utils.svgBarChart('signups', [
      { month: '2026-01', count: 4 },
      { month: '2026-02', count: 8 },
    ], { color: 'var(--accent)' });
    expect(svg).toContain('height="140"');
    expect(svg).toContain('height="70"');
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
      expect(t.active()).toBe(1);
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
    expect(rows[0][1]).toBe('Grace Period');
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
