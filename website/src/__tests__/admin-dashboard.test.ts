// @vitest-environment jsdom
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import vm from 'node:vm';

/**
 * Tests for the static admin dashboard scripts (public/admin/admin-utils.js
 * + admin.js). The pure helpers (escapeHtml, statusPill, fmtIdr/fmtUsd,
 * kpiC, tableCard, svgChart, svgDonut) live in admin-utils.js (loaded first,
 * defines window globals + window.AdminUtils); admin.js consumes them. We
 * load both into a jsdom sandbox in the real script order and exercise the
 * helpers directly. These had zero test coverage despite being the entire
 * rendering surface of the admin panel.
 */

const ADMIN_UTILS_JS = readFileSync(join(process.cwd(), 'public/admin/admin-utils.js'), 'utf8');
const ADMIN_JS = readFileSync(join(process.cwd(), 'public/admin/admin.js'), 'utf8');

/** Build a fresh DOM skeleton + evaluate both admin scripts, returning the helpers. */
function bootAdmin() {
  document.body.innerHTML = `
    <div id="content"></div>
    <div id="modal-root"></div>
    <button id="logout-btn"></button>
    <div id="theme-toggle"></div>
    <button class="nav-btn" data-tab="dashboard"></button>
    <button class="nav-btn" data-tab="tenants"></button>
  `;

  globalThis.fetch = vi.fn(async () => new Response('{}', { status: 200 }));
  window.__OZ_CONFIG__ = { licenseApiUrl: 'https://license.test' };

  // Strip the boot-time renderDashboard() call so admin.js loads without
  // triggering dashboard-render or API calls.
  const adminSrc = ADMIN_JS.replace(/\n\s*renderDashboard\(\);?\s*$/, '');
  // Run in a fresh context per boot so top-level const/let do not collide
  // across boots. Copy the jsdom globals the scripts read.
  const sandbox = vm.createContext({
    window,
    document,
    fetch: globalThis.fetch,
    setTimeout,
    clearTimeout,
    console,
  });
  // Load in the real order: admin-utils.js (defines the helpers as globals
  // via its UMD browser branch) then admin.js (consumes them).
  vm.runInContext(ADMIN_UTILS_JS, sandbox, { filename: 'admin-utils.js' });
  vm.runInContext(adminSrc, sandbox, { filename: 'admin.js' });
  const out: Record<string, unknown> = {};
  for (const name of ['escapeHtml', 'statusPill', 'fmtIdr', 'fmtUsd', 'kpiC', 'tableCard', 'svgChart', 'svgDonut']) {
    out[name] = (sandbox as any)[name];
  }

  return {
    escapeHtml: out.escapeHtml as any,
    statusPill: out.statusPill as any,
    fmtIdr: out.fmtIdr as any,
    fmtUsd: out.fmtUsd as any,
    kpiC: out.kpiC as any,
    tableCard: out.tableCard as any,
    svgChart: out.svgChart as any,
    svgDonut: out.svgDonut as any,
  };
}

beforeEach(() => {
  vi.restoreAllMocks();
  document.body.innerHTML = '';
});

describe('admin.js — escapeHtml', () => {
  it('escapes all five HTML metacharacters', () => {
    const { escapeHtml } = bootAdmin();
    expect(escapeHtml('<script>alert("x") && \'y\'</script>')).toBe(
      '&lt;script&gt;alert(&quot;x&quot;) &amp;&amp; &#39;y&#39;&lt;/script&gt;',
    );
  });

  it('is idempotent-safe for plain strings', () => {
    const { escapeHtml } = bootAdmin();
    expect(escapeHtml('plain text 123')).toBe('plain text 123');
    expect(escapeHtml('')).toBe('');
    expect(escapeHtml(null)).toBe('null');
  });
});

describe('admin.js — statusPill', () => {
  it('maps every known status to a valid pill class', () => {
    const { statusPill } = bootAdmin();
    const cases: Record<string, string> = {
      active: 'pill-ok',
      unused: 'pill-muted',
      grace_period: 'pill-warn',
      expired: 'pill-bad',
      revoked: 'pill-bad',
      paused: 'pill-warn',
      free: 'pill-muted',
      plus: 'pill-ok',
      pro: 'pill-warn',
      premium: 'pill-ok',
      enterprise: 'pill-ok',
    };
    for (const [status, wantCls] of Object.entries(cases)) {
      const pill = statusPill(status);
      expect(pill.className, `status ${status}`).toContain('pill');
      expect(pill.className, `status ${status}`).toContain(wantCls);
      expect(pill.textContent, `status ${status}`).toBe(status);
    }
  });

  it('falls back to muted + em-dash for unknown/undefined status', () => {
    const { statusPill } = bootAdmin();
    const unknown = statusPill('suspended');
    expect(unknown.className).toContain('pill-muted');
    expect(statusPill(undefined).textContent).toBe('—');
  });
});

describe('admin.js — currency formatting', () => {
  it('formats IDR with thousands separators (id-ID)', () => {
    const { fmtIdr } = bootAdmin();
    expect(fmtIdr(16000000)).toContain('16.000.000');
    expect(fmtIdr(0)).toContain('0');
  });

  it('formats USD with exactly two decimals', () => {
    const { fmtUsd } = bootAdmin();
    expect(fmtUsd(1234.5)).toBe('$1234.50');
    expect(fmtUsd(0)).toBe('$0.00');
  });
});

describe('admin.js — kpiC', () => {
  it('builds a KPI card with label, value, and sub', () => {
    const { kpiC } = bootAdmin();
    const card = kpiC('MRR', '$99', 'per subscriber', '<svg/>', 'kpi-icon-blue');
    expect(card.className).toContain('kpi');
    expect(card.querySelector('.kpi-label')?.textContent).toBe('MRR');
    expect(card.querySelector('.kpi-value')?.textContent).toBe('$99');
    expect(card.querySelector('.kpi-sub')?.textContent).toBe('per subscriber');
    expect(card.querySelector('.kpi-icon')?.className).toContain('kpi-icon-blue');
  });

  it('omits the sub row when absent', () => {
    const { kpiC } = bootAdmin();
    const card = kpiC('MRR', '$99', '', '<svg/>');
    expect(card.querySelector('.kpi-sub')).toBeNull();
  });
});

describe('admin.js — tableCard', () => {
  it('renders headers + rows', () => {
    const { tableCard } = bootAdmin();
    const card = tableCard('Top', ['A', 'B'], [['x', '1'], ['y', '2']]);
    expect(card.querySelector('h2')?.textContent).toBe('Top');
    expect(card.querySelectorAll('th')).toHaveLength(2);
    expect(card.querySelectorAll('tbody tr')).toHaveLength(2);
  });

  it('shows an empty state for no rows', () => {
    const { tableCard } = bootAdmin();
    const card = tableCard('Top', ['A', 'B'], []);
    expect(card.querySelector('.empty')?.textContent).toBe('No data.');
    expect(card.querySelector('table')).toBeNull();
  });
});

describe('admin.js — svgChart', () => {
  it('renders a baseline chart for a data series', () => {
    const { svgChart } = bootAdmin();
    const svg = svgChart('rev', [{ month: '2026-01', idr: 100 }, { month: '2026-02', idr: 200 }], ['idr'], { area: true });
    expect(svg).toContain('<svg');
    expect(svg).toContain('<path');
    expect(svg).not.toContain('chart-empty');
  });

  it('returns an empty state for null / empty / all-NaN data (M1 guard)', () => {
    const { svgChart } = bootAdmin();
    expect(svgChart('rev', null as never, ['idr'])).toContain('chart-empty');
    expect(svgChart('rev', [], ['idr'])).toContain('chart-empty');
    expect(svgChart('rev', [{ month: 'x', idr: 'bad' }], ['idr'])).toContain('chart-empty');
  });
});

describe('admin.js — svgDonut', () => {
  it('renders slices + legend with percentages', () => {
    const { svgDonut } = bootAdmin();
    const d = svgDonut('t', [{ tier: 'pro', count: 60 }, { tier: 'plus', count: 40 }], 'tier', 'count');
    expect(d.svg).toContain('<svg');
    expect(d.legend).toContain('pro');
    expect(d.legend).toContain('60%');
    expect(d.legend).toContain('40%');
  });

  it('returns empty state for empty or zero-total data', () => {
    const { svgDonut } = bootAdmin();
    expect(svgDonut('t', [], 'tier', 'count').svg).toContain('chart-empty');
    expect(svgDonut('t', [{ tier: 'pro', count: 0 }], 'tier', 'count').svg).toContain('chart-empty');
  });

  it('escapes legend labels to prevent HTML injection', () => {
    const { svgDonut } = bootAdmin();
    const d = svgDonut('t', [{ tier: '<script>', count: 1 }], 'tier', 'count');
    expect(d.legend).not.toContain('<script>');
    expect(d.legend).toContain('&lt;script&gt;');
  });
});
