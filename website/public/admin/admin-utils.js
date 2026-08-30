// OZ-POS Admin — pure helper module (H1/H2 hardening).
//
// Extracted from the monolithic admin.js so the chart/format/escape logic
// is unit-testable in isolation (vitest, jsdom). Loaded as a plain script
// BEFORE admin.js; defines window.AdminUtils AND individual globals for
// backward compatibility with admin.js call sites.
//
// In Node (vitest): module.exports for direct import.
(function (root, factory) {
  if (typeof module === 'object' && module.exports) {
    module.exports = factory(); // Node/vitest
  } else {
    var utils = factory();      // browser
    root.AdminUtils = utils;
    // Also set individual globals so admin.js (which calls bare function
    // names like el(), escapeHtml()) works without changes.
    Object.keys(utils).forEach(function (k) { root[k] = utils[k]; });
  }
}(typeof self !== 'undefined' ? self : this, function () {
  'use strict';

  // Create a DOM element safely (never innerHTML with API data).
  function el(tag, cls, text) {
    var e = document.createElement(tag);
    if (cls) e.className = cls;
    if (text !== undefined) e.textContent = text;
    return e;
  }

  // Escape HTML entities for any API-sourced string interpolated into
  // innerHTML (defense-in-depth — donut legend labels, chart text).
  function escapeHtml(s) {
    return String(s).replace(/[&<>"']/g, function (ch) {
      return ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[ch];
    });
  }

  function fmtIdr(val) { return 'Rp ' + Math.round(val).toLocaleString('id-ID'); }
  function fmtUsd(val) { return '$' + Number(val).toFixed(2); }

  function statusPill(status) {
    var map = { active: ['pill-ok'], unused: ['pill-muted'], grace_period: ['pill-warn'], expired: ['pill-bad'], revoked: ['pill-bad'], paused: ['pill-warn'], free: ['pill-muted'], plus: ['pill-ok'], pro: ['pill-warn'], premium: ['pill-ok'], enterprise: ['pill-ok'] };
    var cls = (map[status] || ['pill-muted'])[0];
    return el('span', 'pill ' + cls, status || '—');
  }

  // svgChart renders a multi-series line chart as an SVG string. Pure:
  // takes id (unused, kept for signature compat), data + series + opts and
  // returns an SVG string (no DOM access).
  function svgChart(id, data, series, opts) {
    if (!data || !Array.isArray(data) || data.length === 0) {
      return '<div class="chart-empty">No data</div>';
    }
    var vals = data.map(function (d) { return series.map(function (s) { return Number(d[s]); }); }).flat().filter(function (n) { return Number.isFinite(n); });
    if (vals.length === 0) {
      return '<div class="chart-empty">No data</div>';
    }
    var w = 600, h = 180, px = 40, py = 20, pw = w - px, ph = h - py - 20;
    var max = Math.max.apply(null, vals);
    var min = 0;
    var rng = max - min || 1;
    var x = function (i) { return px + (i / (data.length - 1 || 1)) * pw; };
    var y = function (v) { return py + ph - ((v - min) / rng) * ph; };
    var colors = { usd: '#147efb', idr: '#22c55e', count: '#147efb', mrr: '#147efb' };
    var paths = '', fills = '';
    series.forEach(function (s) {
      var pts = data.map(function (d, i) { return x(i) + ',' + y(Number(d[s]) || 0); }).join(' L ');
      paths += '<path d="M ' + pts + '" stroke="' + (colors[s] || '#147efb') + '" stroke-width="2" fill="none" class="chart-line"/>';
      if (opts && opts.area) {
        var base = x(0) + ',' + (py + ph) + ' L ' + pts + ' L ' + x(data.length - 1) + ',' + (py + ph) + ' Z';
        fills += '<path d="' + base + '" fill="' + (colors[s] || '#147efb') + '" opacity=".08"/>';
      }
    });
    var yLabels = '';
    for (var i = 0; i <= 4; i++) {
      var v = min + (rng / 4) * i;
      yLabels += '<text x="' + (px - 5) + '" y="' + (y(v) + 3) + '" text-anchor="end" fill="var(--muted)" font-size="10">' + (opts && opts.fmt ? opts.fmt(v) : Math.round(v)) + '</text>';
    }
    var xLabels = '';
    data.forEach(function (d, i) {
      if (i % 2 === 0 || i === data.length - 1) {
        xLabels += '<text x="' + x(i) + '" y="' + (py + ph + 15) + '" text-anchor="middle" fill="var(--muted)" font-size="9">' + d.month.slice(5) + '</text>';
      }
    });
    return '<svg viewBox="0 0 ' + w + ' ' + h + '" class="chart-svg">' + fills + paths + yLabels + xLabels + '</svg>';
  }

  // svgDonut renders a donut chart + legend. Pure; guards empty/zero data.
  function svgDonut(id, data, labelKey, valueKey, colors) {
    if (!data || !Array.isArray(data) || data.length === 0) {
      return { svg: '<div class="chart-empty">No data</div>', legend: '' };
    }
    var total = data.reduce(function (s, d) { return s + (Number(d[valueKey]) || 0); }, 0);
    if (total <= 0) {
      return { svg: '<div class="chart-empty">No data</div>', legend: '' };
    }
    var acc = 0;
    var slices = '';
    var cx = 80, cy = 80, r = 60;
    var colorList = ['#147efb', '#22c55e', '#e879f9', '#fb923c', '#22d3ee', '#f59e0b'];
    data.forEach(function (d, i) {
      var pct = (Number(d[valueKey]) || 0) / total;
      var ang = pct * 360;
      var start = (acc / 360) * 2 * Math.PI - Math.PI / 2;
      var end = ((acc + ang) / 360) * 2 * Math.PI - Math.PI / 2;
      var x1 = cx + r * Math.cos(start), y1 = cy + r * Math.sin(start);
      var x2 = cx + r * Math.cos(end), y2 = cy + r * Math.sin(end);
      var large = ang > 180 ? 1 : 0;
      var c = colors && colors[i] ? colors[i] : colorList[i % colorList.length];
      slices += '<path d="M ' + cx + ' ' + cy + ' L ' + x1 + ' ' + y1 + ' A ' + r + ' ' + r + ' 0 ' + large + ' 1 ' + x2 + ' ' + y2 + ' Z" fill="' + c + '" stroke="var(--bg)" stroke-width="2"/>';
      acc += ang;
    });
    var legend = '';
    data.forEach(function (d, i) {
      var pct = (Number(d[valueKey]) || 0) / total;
      var c = colors && colors[i] ? colors[i] : colorList[i % colorList.length];
      legend += '<div class="donut-legend-item"><span class="donut-swatch" style="background:' + c + '"></span><span class="donut-label">' + escapeHtml(d[labelKey]) + '</span> <span class="donut-pct">' + Math.round(pct * 100) + '%</span></div>';
    });
    return { svg: '<svg viewBox="0 0 160 160">' + slices + '</svg>', legend: legend };
  }

  // kpiC builds a KPI stat card (label + value + optional sub + icon).
  // Pure DOM helper — unit-testable in jsdom.
  function kpiC(label, value, sub, icon, iconCls) {
    var s = el('div', 'kpi');
    if (icon) {
      var ic = el('div', 'kpi-icon ' + (iconCls || 'kpi-icon-blue'));
      ic.innerHTML = icon;
      s.appendChild(ic);
    }
    var body = el('div', 'kpi-body');
    body.appendChild(el('div', 'kpi-label', label));
    body.appendChild(el('div', 'kpi-value', value));
    if (sub) body.appendChild(el('div', 'kpi-sub', sub));
    s.appendChild(body);
    return s;
  }

  // tableCard builds a card with a header + data table (or an empty state).
  // rows is an array of arrays of cell strings.
  function tableCard(heading, headers, rows) {
    var card = el('div', 'card table-card');
    card.appendChild(el('h2', null, heading));
    if (!rows || rows.length === 0) { card.appendChild(el('p', 'empty', t('table.noData'))); return card; }
    var table = el('table');
    var thead = el('thead');
    var tr = el('tr');
    headers.forEach(function (h) { tr.appendChild(el('th', null, h)); });
    thead.appendChild(tr); table.appendChild(thead);
    var tbody = el('tbody');
    rows.forEach(function (row) {
      var tr2 = el('tr');
      row.forEach(function (cell) { tr2.appendChild(el('td', null, cell)); });
      tbody.appendChild(tr2);
    });
    table.appendChild(tbody);
    card.appendChild(table);
    return card;
  }

  // ── API helper (H1) ─────────────────────────────────────────────
  // Pure classification + error-builder for the admin API layer. The DOM
  // side (rendering the access-denied screen) stays in admin.js; these are
  // unit-testable without a fetch/document.

  // Returns true when an HTTP status means "session not authorized" for the
  // admin panel (401 unauth'd, 403 non-admin tenant).
  function isAuthDenied(status) {
    return status === 401 || status === 403;
  }

  // Build the thrown Error for an auth-denied response. Marking
  // err.authDenied lets callers distinguish "fetch failed" from "auth was
  // the problem" so they never overwrite the access-denied screen.
  function authDeniedError(path) {
    const err = new Error(path + ' (auth denied)');
    err.authDenied = true;
    return err;
  }

  // ── i18n (H3) ───────────────────────────────────────────────────
  // Key-value string table. English is the default; a future locale just
  // swaps this object. t(key) returns the localized string (missing keys
  // fall back to the key itself so the UI never shows a blank label).
  var STRINGS = {
    'dashboard.title': 'Dashboard',
    'kpi.totalUsers': 'Total Users',
    'kpi.totalSubscribers': 'Total Subscribers',
    'kpi.mrr': 'MRR',
    'kpi.monthlyGrossIdr': 'Monthly Gross (IDR)',
    'kpi.arpu': 'ARPU',
    'kpi.activeTerminals': 'Active Terminals',
    'kpi.trialToPaid': 'Trial → Paid',
    'chart.revenueTrendIdr': 'Revenue Trend (IDR)',
    'chart.subscriberGrowth': 'Subscriber Growth',
    'chart.tierDistribution': 'Tier Distribution',
    'chart.paymentProvider': 'Payment Provider',
    'chart.signupsPerMonth': 'Signups per Month',
    'chart.churnCanceled': 'Churn / Canceled',
    'table.topSubscribers': 'Top Subscribers',
    'table.recentSignups': 'Recent Signups',
    'table.expiringSoon': 'Expiring Soon (within 30 days)',
    'table.tenants': 'Tenants',
    'table.noData': 'No data.',
    'table.noTenantsMatch': 'No tenants match.',
    'th.email': 'Email',
    'th.tier': 'Tier',
    'th.mrr': 'MRR',
    'th.renewal': 'Renewal',
    'th.provider': 'Provider',
    'th.status': 'Status',
    'th.daysLeft': 'Days Left',
    'th.created': 'Created',
    'th.expires': 'Expires',
    'th.licenseKey': 'License key',
    'th.license': 'License',
    'th.devices': 'Devices',
    'th.subscriptionStatus': 'Subscription status',
    'th.emailVerified': 'Email verified',
    'tenant.currentTier': 'Current tier: ',
    'tenant.details': 'Details',
    'tenant.title': 'Tenant: ',
    'tenant.changeTier': 'Change tier',
    'tenant.reasonOverride': 'Reason for override (audit)',
    'tenant.renew365': 'Renew +365d',
    'tenant.revoke': 'Revoke',
    'tenant.revoked': 'Revoked',
    'tenant.activate': 'Activate',
    'tenant.activated': 'Activated',
    'tenant.renewed': 'Renewed',
    'tenant.tierChanged': 'Tier changed',
    'tenant.upgrade': 'Upgrade',
    'tenant.save': 'Save',
    'tenant.cancel': 'Cancel',
    'tenant.close': 'Close',
    'toolbar.searchPlaceholder': 'Search by email…',
    'toolbar.search': 'Search',
    'toolbar.clear': 'Clear',
    'toolbar.enter': 'Enter',
    'toolbar.showing': 'Showing ',
    'toolbar.of': ' of ',
    'toolbar.page': 'Page ',
    'toolbar.prev': '← Prev',
    'toolbar.next': 'Next →',
    'health.title': 'System Health',
    'health.status': 'Status',
    'health.ok': '✓ OK',
    'health.degraded': '✗ Degraded',
    'health.database': 'Database',
    'health.connected': '✓ Connected',
    'health.unreachable': '✗ Unreachable',
    'health.smtp': 'SMTP',
    'health.configured': '✓ Configured',
    'health.notConfigured': '— Not configured',
    'health.version': 'Version',
    'health.time': 'Time',
    'common.loading': 'Loading…',
    'common.loadingTenants': 'Loading tenants…',
    'common.failedToLoadTenants': 'Failed to load tenants.',
    'common.failedToLoadTenantDetail': 'Failed to load tenant detail.',
    'common.failedToLoadHealth': 'Failed to load health.',
    'common.statsUnavailable': 'Stats unavailable',
    'common.statsApiNoResponse': 'The dashboard API did not respond. Try again.',
    'common.retry': 'Retry',
    'common.stale': ' stale',
    'common.successfully': ' successfully',
    'common.failed': ' failed',
    'toolbar.nonFree': 'non-free (plus/pro/premium/enterprise)',
    'toolbar.perSubscriber': 'per subscriber',
    'toolbar.conversionRate': 'conversion rate',
    // ── Login pages (shared by admin + dashboard login.js) ──
    'login.sendCode': 'Send Verification Code',
    'login.sendingCode': 'Sending verification code…',
    'login.verifyCode': 'Verify Code & Sign In',
    'login.verifyingCode': 'Verifying code…',
    'login.signIn': 'Sign In',
    'login.signInPassword': 'Sign In with Password',
    'login.signingIn': 'Signing in…',
    'login.enterEmail': 'Please enter your email address',
    'login.enterCode': 'Please enter the 6-digit code from your email',
    'login.enterPassword': 'Please enter your password',
    'login.invalidOrExpiredCode': 'Invalid or expired verification code',
    'login.invalidEmailOrPassword': 'Invalid email or password',
    'login.failedToSendCode': 'Failed to send verification code',
    'login.emailDeliveryNotConfigured': 'Email delivery is not configured on server',
    'login.accessDeniedOrigin': 'Access denied: origin not allowed',
    'login.couldNotConnect': 'Could not connect to authentication server',
    'login.resendPrompt': 'Did not receive code? Click below to resend.',
    'login.signingIn': 'Signing in…',
    'login.resendIn': 'Resend code in ',
    'login.tryAgainIn': 'Try again in ',
    'login.seconds': 's',
    'login.codeSent': '✓ Verification code sent! Please check your email inbox (and spam folder).',
    'login.codeVerified': '✓ Code verified! Signing in…',
    'login.signingInNow': '✓ Signing in…',
    'login.invalidOrExpiredCodeShort': 'Invalid or expired code',
    'auth.accessDenied': 'Access denied',
    'auth.signInAgain': 'Your session is not authorized for the admin panel. If you are the admin, please <a href="/__oz/logout" style="color:var(--accent)">sign in again</a>.',
  };

  function t(key) {
    return STRINGS[key] !== undefined ? STRINGS[key] : key;
  }

  return {
    el: el,
    escapeHtml: escapeHtml,
    fmtIdr: fmtIdr,
    fmtUsd: fmtUsd,
    statusPill: statusPill,
    svgChart: svgChart,
    svgDonut: svgDonut,
    kpiC: kpiC,
    tableCard: tableCard,
    t: t,
    STRINGS: STRINGS,
    isAuthDenied: isAuthDenied,
    authDeniedError: authDeniedError,
  };
}));