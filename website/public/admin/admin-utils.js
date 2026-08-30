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

  // statusLabel maps the server's status enum to a human label (B16).
  // Unknown values fall back to the raw string — never to a missing-key
  // placeholder, so a new server-side status still shows something real.
  function statusLabel(status) {
    if (status === undefined || status === null || status === '') return '—';
    var key = 'status.' + status;
    return STRINGS[key] !== undefined ? STRINGS[key] : status;
  }

  function statusPill(status) {
    var map = { active: ['pill-ok'], unused: ['pill-muted'], grace_period: ['pill-warn'], expired: ['pill-bad'], revoked: ['pill-bad'], paused: ['pill-warn'], free: ['pill-muted'], plus: ['pill-ok'], pro: ['pill-warn'], premium: ['pill-ok'], enterprise: ['pill-ok'] };
    var cls = (map[status] || ['pill-muted'])[0];
    return el('span', 'pill ' + cls, statusLabel(status));
  }

  // createSeqGuard tracks the newest of N overlapping async requests so
  // superseded responses can be discarded (B15: renderTenants let a slow
  // page-2 response overwrite page 3 — last-arrival-wins, not
  // last-click-wins). next() stamps a request; isCurrent(id) is true only
  // for the most recent stamp.
  function createSeqGuard() {
    var n = 0;
    return {
      next: function () { n += 1; return n; },
      isCurrent: function (id) { return id === n; },
    };
  }

  // svgChart renders a multi-series line chart as an SVG string. Pure:
  // takes _id (unused, kept for signature compatibility with call sites
  // that pass a chart id), data + series + opts and returns an SVG string.
  function svgChart(_id, data, series, opts) {
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
        // B5 fix: the M1 guard protected values but not labels — a row
        // without month threw on .slice and killed the whole dashboard.
        xLabels += '<text x="' + x(i) + '" y="' + (py + ph + 15) + '" text-anchor="middle" fill="var(--muted)" font-size="9">' + escapeHtml(d.month ? String(d.month).slice(5) : '') + '</text>';
      }
    });
    return '<svg viewBox="0 0 ' + w + ' ' + h + '" class="chart-svg">' + fills + paths + yLabels + xLabels + '</svg>';
  }

  // svgDonut renders a donut chart + legend. Pure; guards empty/zero data.
  // _id is unused (kept for signature compatibility).
  function svgDonut(_id, data, labelKey, valueKey, colors) {
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
      if (ang >= 360) {
        // B4 fix: a single arc with start == end point draws NOTHING per
        // the SVG spec — a 100% slice (all tenants on one tier, the common
        // early state) rendered as an empty ring while the legend claimed
        // 100%. A full circle needs two arcs; split at the halfway angle.
        var mid = start + Math.PI;
        var xm = cx + r * Math.cos(mid), ym = cy + r * Math.sin(mid);
        slices += '<path d="M ' + cx + ' ' + cy + ' L ' + x1 + ' ' + y1 + ' A ' + r + ' ' + r + ' 0 0 1 ' + xm + ' ' + ym + ' A ' + r + ' ' + r + ' 0 0 1 ' + x1 + ' ' + y1 + ' Z" fill="' + c + '" stroke="var(--bg)" stroke-width="2"/>';
      } else {
        slices += '<path d="M ' + cx + ' ' + cy + ' L ' + x1 + ' ' + y1 + ' A ' + r + ' ' + r + ' 0 ' + large + ' 1 ' + x2 + ' ' + y2 + ' Z" fill="' + c + '" stroke="var(--bg)" stroke-width="2"/>';
      }
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

  // tenantRow builds one <tr> for the tenants table (email, status pill,
  // license key, tier, created date, Details button). Extracted from
  // admin.js renderTenants so the row logic is unit-testable.
  // INVARIANT: the parameter must NEVER be named `t` — it would shadow the
  // i18n t() helper (the B1 bug: the i18n refactor left `t('tenant.details')`
  // calling a tenant object, crashing the whole Tenants tab).
  function tenantRow(tenant, onDetails) {
    var row = el('tr');
    row.appendChild(el('td', null, tenant.email || '—'));
    var td1 = el('td'); td1.appendChild(statusPill(tenant.status)); row.appendChild(td1);
    row.appendChild(el('td', null, (tenant.license && tenant.license.key) || '—'));
    row.appendChild(el('td', null, (tenant.subscription && tenant.subscription.tierKey) || '—'));
    row.appendChild(el('td', null, tenant.created ? tenant.created.slice(0, 10) : '—'));
    var tdAction = el('td');
    var btn = el('button', 'btn btn-sm btn-ghost', t('tenant.details'));
    btn.addEventListener('click', function () { onDetails(tenant.id); });
    tdAction.appendChild(btn); row.appendChild(tdAction);
    return row;
  }

  // tenantDetailRows builds the [label, value] pairs for the tenant
  // detail modal's key/value grid (extracted from admin.js
  // showTenantDetail so the data mapping is unit-testable).
  function tenantDetailRows(data) {
    var tenant = data.tenant || {}, lic = data.license || {}, sub = data.subscription || {}, devices = data.devices || [];
    return [
      [t('th.status'), statusLabel(tenant.status)],
      [t('th.emailVerified'), tenant.emailVerified ? '✓' : '○'],
      [t('th.created'), tenant.created ? tenant.created.slice(0, 10) : '—'],
      [t('th.licenseKey'), lic.key || '—'],
      [t('th.tier'), sub.tierKey || lic.tierKey || '—'],
      [t('th.subscriptionStatus'), statusLabel(sub.status)],
      [t('th.expires'), sub.expiresAt || '—'],
      [t('th.devices'), devices.length],
    ];
  }

  // svgBarChart renders a simple bar chart as an SVG string (extracted
  // from the signups/churn blocks in admin.js renderDashboard).
  // B3 fix: the value field is opts.valueKey (default 'count') — the
  // server's churnPerMonth rows carry the number in `churn` with `count`
  // at Go's zero value, so a hardcoded d.count rendered permanently-zero
  // (NaN-height) churn bars. Empty data gets the chart-empty state
  // instead of Math.max()=-Infinity / 420/0=Infinity geometry, and a
  // missing month degrades to '' instead of throwing.
  function svgBarChart(_id, data, opts) {
    if (!data || !Array.isArray(data) || data.length === 0) {
      return '<div class="chart-empty">No data</div>';
    }
    var valueKey = (opts && opts.valueKey) || 'count';
    var maxS = Math.max.apply(null, data.map(function (d) { return Number(d[valueKey]) || 0; }));
    var barW = 420 / data.length;
    var bars = '';
    data.forEach(function (d, i) {
      var v = Number(d[valueKey]) || 0;
      var bh = maxS > 0 ? (v / maxS) * 140 : 0;
      var bx = 10 + i * (barW + 2);
      bars += '<rect x="' + bx + '" y="' + (150 - bh) + '" width="' + (barW * 0.7) + '" height="' + bh + '" rx="2" fill="' + (opts && opts.color || 'var(--accent)') + '" opacity=".8"/>' +
        '<text x="' + (bx + barW * 0.35) + '" y="' + (150 - bh - 4) + '" text-anchor="middle" fill="var(--text)" font-size="9">' + v + '</text>' +
        '<text x="' + (bx + barW * 0.35) + '" y="165" text-anchor="middle" fill="var(--muted)" font-size="8">' + escapeHtml(d.month ? d.month.slice(5) : '') + '</text>';
    });
    return '<svg viewBox="0 0 440 180" style="max-height:180px">' + bars + '</svg>';
  }

  // timeoutSignal builds an AbortSignal for a request, degrading to NO
  // signal where AbortSignal.timeout is unavailable (B20: the static is
  // Chrome/WebView 103+/Safari 16+; calling it unguarded threw TypeError
  // on older browsers and broke EVERY api() call — a regression
  // introduced by the B10/B12 timeout fixes). Un-timed beats broken.
  function timeoutSignal(timeoutMs, fallbackMs) {
    var ms = timeoutMs > 0 ? timeoutMs : fallbackMs;
    if (typeof AbortSignal !== 'undefined' && AbortSignal.timeout) {
      return AbortSignal.timeout(ms);
    }
    return undefined;
  }

  // fetchFxRate queries open.er-api.com for the live USD→IDR rate.
  // Extracted from admin.js so the timeout semantics are unit-testable.
  // B10 fix: the original awaited an UN-TIMED fetch — a firewalled or
  // captive-portal-blocked er-api.com left the dashboard skeleton hanging
  // for the browser's full connect timeout. The request now carries an
  // AbortSignal.timeout and any failure degrades to {live:false}, so the
  // caller falls back to the last-known/default rate immediately.
  async function fetchFxRate(fetchImpl, timeoutMs) {
    var f = fetchImpl || fetch;
    try {
      var r = await f('https://open.er-api.com/v6/latest/USD', {
        signal: timeoutSignal(timeoutMs, 5000),
      });
      var d = await r.json();
      var idr = d && d.rates ? Number(d.rates.IDR) : NaN;
      if (Number.isFinite(idr) && idr > 0) {
        return { rate: idr, updatedAt: new Date().toISOString(), live: true };
      }
    } catch (e) { /* not live */ }
    return { rate: null, updatedAt: '', live: false };
  }

  // exchangeUrlFrom validates the /exchange-issue response and builds the
  // redirect URL. B13 fix: login.js concatenated body.code unguarded — a
  // 200 response without a code sent the browser to /?code=undefined, the
  // worker's consume failed, and it bounced back to login: a silent loop.
  function exchangeUrlFrom(body) {
    if (!body || typeof body.code !== 'string' || body.code === '') {
      var err = new Error('exchange-issue returned no code');
      err.exchangeFailed = true;
      throw err;
    }
    return '/?code=' + encodeURIComponent(body.code);
  }

  // isLockoutActive reports whether a button currently carries a running
  // startLockoutCountdown timer. B14: login.js setAuthMode must not
  // overwrite the countdown label mid-lockout.
  function isLockoutActive(btn) {
    return !!(btn && btn._ozLockoutTimer);
  }

  // startCountdown / stopCountdown / countdownActive — generic per-node
  // countdown timer. B18 fix: login.js startOtpCooldown kept the resend
  // cooldown in a module global. Switching auth mode (setAuthMode) hid the
  // cooldown <span> without clearing the timer; switching back to OTP mode
  // skipped showing it, so a live 60s cooldown ran invisibly and the user
  // hit a 429 on the next resend. Timer is now stored on the DOM node
  // (_ozCdTimer) so visibility can follow countdownActive(node) and
  // stopCountdown() can tear it down from anywhere.
  /**
   * @param {HTMLElement|null} node
   * @param {number} seconds
   * @param {function(number): string} fmt
   * @param {function(): void} onEnd
   */
  function startCountdown(node, seconds, fmt, onEnd) {
    if (!node) return;
    if (node._ozCdTimer) clearInterval(node._ozCdTimer);
    var remaining = seconds;
    node.textContent = fmt(remaining);
    node._ozCdTimer = setInterval(function () {
      remaining -= 1;
      if (remaining <= 0) {
        clearInterval(node._ozCdTimer);
        node._ozCdTimer = null;
        onEnd();
        return;
      }
      node.textContent = fmt(remaining);
    }, 1000);
  }

  function stopCountdown(node) {
    if (!node) return;
    if (node._ozCdTimer) {
      clearInterval(node._ozCdTimer);
      node._ozCdTimer = null;
    }
  }

  function countdownActive(node) {
    return !!(node && node._ozCdTimer);
  }

  // busyWrap — single-flight async guard for action buttons (B19).
  function busyWrap(btn, fn) {
    var busy = false;
    return function () {
      if (busy) return Promise.resolve();
      busy = true;
      if (btn) btn.disabled = true;
      var result;
      try {
        result = fn();
      } catch (e) {
        busy = false;
        if (btn) btn.disabled = false;
        throw e;
      }
      if (!result || typeof result.then !== 'function') {
        busy = false;
        if (btn) btn.disabled = false;
        return result;
      }
      return result.then(
        function (v) { busy = false; if (btn) btn.disabled = false; return v; },
        function (e) { busy = false; if (btn) btn.disabled = false; return Promise.reject(e); }
      );
    };
  }

  // fetchWithTimeout performs a fetch that is guaranteed to settle.
  // Extracted from admin.js api() so the timeout semantics are testable.
  // B12 fix: api() awaited two UN-TIMED fetches per call (the session
  // endpoint and the license API). A hung license-server connection —
  // half-open TCP, overloaded origin — left renderDashboard/renderTenants/
  // renderHealth pending forever: skeleton, no retry UI, no console error.
  // Every admin fetch now carries AbortSignal.timeout; a timeout surfaces
  // as a rejection the existing catch paths already render as errors.
  async function fetchWithTimeout(fetchImpl, url, opts, timeoutMs) {
    var f = fetchImpl || fetch;
    var o = opts || {};
    var sig = timeoutSignal(timeoutMs, 15000);
    if (sig) o.signal = sig;
    return f(url, o);
  }

  // mountModal wires the shared modal mechanics: backdrop + dialog box,
  // backdrop-click close, ESC close, and a returned close() for buttons.
  // Extracted from the duplicated blocks in admin.js showTenantDetail /
  // upgradePrompt so the listener lifecycle is unit-testable.
  // B11 fix: the original only detached the keydown handler on the ESC
  // path — closing via the button or backdrop left it attached, so every
  // such open leaked one listener that kept reacting to later ESCs
  // (clearing whatever modal was open then, and double-firing). One
  // idempotent close() now serves all three paths and always detaches.
  function mountModal(modalRoot, box) {
    var m = el('div', 'modal-back');
    m.appendChild(box);
    var open = true;
    function close() {
      if (!open) return;
      open = false;
      // Only clear the root while our backdrop is still mounted — a
      // stacked modal (upgrade over detail) may already have replaced it.
      if (m.parentNode === modalRoot) modalRoot.innerHTML = '';
      document.removeEventListener('keydown', escHandler);
    }
    function escHandler(e) { if (e.key === 'Escape') close(); }
    m.addEventListener('click', function (e) { if (e.target === m) close(); });
    document.addEventListener('keydown', escHandler);
    modalRoot.appendChild(m);
    return close;
  }

  // startCountdown drives a plain text node with a per-second countdown
  // (the OTP resend cooldown). B18: the original kept its timer in a
  // login.js module global and setAuthMode hid the element without
  // touching the timer — a live cooldown ran invisibly after a tab
  // switch and the user walked into a 429. The handle now lives on the
  // node (same pattern as startLockoutCountdown), so visibility can
  // follow countdownActive(node).
  function startCountdown(node, seconds, fmt, onEnd) {
    if (!node) return;
    if (node._ozCdTimer) clearInterval(node._ozCdTimer);
    var remaining = seconds;
    node.textContent = fmt(remaining);
    node._ozCdTimer = setInterval(function () {
      remaining -= 1;
      if (remaining <= 0) {
        clearInterval(node._ozCdTimer);
        node._ozCdTimer = null;
        if (onEnd) onEnd();
      } else {
        node.textContent = fmt(remaining);
      }
    }, 1000);
  }

  function stopCountdown(node) {
    if (node && node._ozCdTimer) { clearInterval(node._ozCdTimer); node._ozCdTimer = null; }
  }

  function countdownActive(node) {
    return !!(node && node._ozCdTimer);
  }

  // setAuthMode applies the login form's OTP/password tab state.
  // Extracted from login.js so the mode-switch guards are unit-testable.
  // B21 fix: the tab buttons called it unconditionally — clicking the
  // other tab while a login request was in flight flipped currentMode,
  // and the response handler then wrote the WRONG mode's button label
  // (and could start the OTP cooldown on the password tab). opts
  // .isSubmitting() lets the caller veto the flip mid-submit; the
  // function returns whether it applied so login.js can gate currentMode.
  function setAuthMode(mode, els, opts) {
    els = els || {};
    if (opts && typeof opts.isSubmitting === 'function' && opts.isSubmitting()) return false;
    var tabOtp = els.tabOtp, tabPwd = els.tabPwd, pwdGroup = els.pwdGroup,
        otpGroup = els.otpGroup, loginBtn = els.loginBtn, cd = els.cd;
    if (mode === 'otp') {
      if (tabOtp) tabOtp.classList.add('active');
      if (tabPwd) tabPwd.classList.remove('active');
      if (pwdGroup) pwdGroup.classList.add('hidden');
      // B18: the resend cooldown is enforced server-side and its countdown
      // keeps running across a tab switch — re-show it when returning.
      if (cd && countdownActive(cd)) cd.classList.remove('hidden');
      var isCodeActive = otpGroup && !otpGroup.classList.contains('hidden');
      // B14: never overwrite an active lockout countdown label.
      if (loginBtn && !isLockoutActive(loginBtn)) {
        if (isCodeActive) {
          loginBtn.textContent = t('login.verifyCode');
        } else {
          if (otpGroup) otpGroup.classList.add('hidden');
          loginBtn.textContent = t('login.sendCode');
        }
      } else if (!isCodeActive && otpGroup) {
        otpGroup.classList.add('hidden');
      }
    } else {
      if (tabOtp) tabOtp.classList.remove('active');
      if (tabPwd) tabPwd.classList.add('active');
      if (pwdGroup) pwdGroup.classList.remove('hidden');
      if (otpGroup) otpGroup.classList.add('hidden');
      if (cd) cd.classList.add('hidden');
      if (loginBtn && !isLockoutActive(loginBtn)) loginBtn.textContent = t('login.signInPassword');
    }
    return true;
  }

  // startLockoutCountdown drives the login button's 429 lockout label.
  // Extracted from login.js showLockoutCountdown so the timer semantics
  // are unit-testable (login.js has DOM boot side effects).
  // B7 fix: the original created a NEW setInterval per 429 and only ever
  // referenced it from its own closure — a second rate-limited response
  // left the first timer racing: it re-enabled the button early (its
  // shorter retry_after expired first) and zombie-rewrote the restored
  // label afterwards. The handle now lives on the button, so a new
  // countdown always supersedes the previous one.
  /**
   * @param {HTMLElement|null} btn
   * @param {number} seconds
   * @param {function(number): string} fmt
   * @param {function(): string} restore
   */
  function startLockoutCountdown(btn, seconds, fmt, restore) {
    if (!btn) return;
    if (btn._ozLockoutTimer) clearInterval(btn._ozLockoutTimer);
    btn.disabled = true;
    var remaining = seconds;
    btn.textContent = fmt(remaining);
    btn._ozLockoutTimer = setInterval(function () {
      remaining--;
      if (remaining <= 0) {
        clearInterval(btn._ozLockoutTimer);
        btn._ozLockoutTimer = null;
        btn.disabled = false;
        btn.textContent = restore();
        return;
      }
      btn.textContent = fmt(remaining);
    }, 1000);
  }

  /**
   * @typedef {Object} AdminKpis
   * @property {number} mrrUsd
   * @property {number} totalUsers
   * @property {number} arpuUsd
   * @property {number} fxRate
   * @property {number} activeUsers
   * @property {number} totalSubscribers
   * @property {number} activeDevices
   * @property {number} trialToPaidRate
   */

  /**
   * @typedef {Object} AdminStats
   * @property {Array} revenueTrend
   * @property {Array} subscriberGrowth
   * @property {Array} tierDistribution
   * @property {Array} providerSplit
   * @property {Array} signupsPerMonth
   * @property {Array} churnPerMonth
   * @property {Array} topSubscribers
   * @property {Array} recentSignups
   * @property {Array} expiringSoon
   * @property {AdminKpis} kpis
   */

  // normalizeStats guarantees the shapes renderDashboard expects. admin.js
  // dereferenced m.revenueTrend.forEach / m.kpis.mrrUsd BEFORE the chart
  // guards ran, so a partial stats payload (older server build, truncated
  // response) threw a bare TypeError and the dashboard rendered nothing.
  // Non-object input is tolerated; numeric KPIs are coerced (null/undefined
  // -> 0, numeric strings -> number) so fmtUsd/fmtIdr never see NaN text.
  /**
   * @param {any} raw
   * @returns {AdminStats}
   */
  function normalizeStats(raw) {
    var m = (raw && typeof raw === 'object') ? raw : {};
    var arr = function (v) { return Array.isArray(v) ? v : []; };
    var num = function (v) { var n = Number(v); return Number.isFinite(n) ? n : 0; };
    var k = (m.kpis && typeof m.kpis === 'object') ? m.kpis : {};
    var kpis = {};
    Object.keys(k).forEach(function (key) { kpis[key] = k[key]; });
    ['totalUsers', 'activeUsers', 'totalSubscribers', 'activeDevices', 'mrrUsd', 'arpuUsd', 'trialToPaidRate']
      .forEach(function (key) { kpis[key] = num(k[key]); });
    return {
      revenueTrend: arr(m.revenueTrend),
      subscriberGrowth: arr(m.subscriberGrowth),
      tierDistribution: arr(m.tierDistribution),
      providerSplit: arr(m.providerSplit),
      signupsPerMonth: arr(m.signupsPerMonth),
      churnPerMonth: arr(m.churnPerMonth),
      topSubscribers: arr(m.topSubscribers),
      recentSignups: arr(m.recentSignups),
      expiringSoon: arr(m.expiringSoon),
      kpis: kpis,
    };
  }

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
    'login.exchangeFailed': 'Sign-in succeeded but the session handoff failed. Please try again.',
    // B16: human labels for the server status enum (PocketBase
    // SelectField values: active/expired/grace_period/revoked/paused,
    // plus 'unused' for license keys). Unknown values fall back to raw.
    'status.active': 'Active',
    'status.expired': 'Expired',
    'status.grace_period': 'Grace Period',
    'status.revoked': 'Revoked',
    'status.paused': 'Paused',
    'status.unused': 'Unused',
    'login.resendPrompt': 'Did not receive code? Click below to resend.',
    'login.signingIn': 'Signing in…',
    'login.resendIn': 'Resend code in ',
    'login.tryAgainIn': 'Try again in ',
    'login.seconds': 's',
    'login.codeSent': '✓ Verification code sent! Please check your email inbox (and spam folder).',
    'login.codeVerified': '✓ Code verified! Signing in…',
    'login.signingInNow': '✓ Signing in…',
    'login.invalidOrExpiredCodeShort': 'Invalid or expired code',
    // ── User dashboard SPA (dashboard.js) ──
    'dash.account': 'Account',
    'dash.dashboard': 'Dashboard',
    'dash.yourAccount': 'Your account at a glance',
    'dash.subscription': 'Subscription',
    'dash.license': 'License',
    'dash.devices': 'Devices',
    'dash.maxStores': 'Max Stores',
    'dash.maxTerminal': 'Max Terminal',
    'dash.maxKds': 'Max KDS',
    'dash.stores': 'stores',
    'dash.registers': 'registers',
    'dash.screens': 'screens',
    'dash.currentPlan': 'current plan',
    'dash.copy': 'Copy',
    'dash.copied': '✓ Copied',
    'dash.noActiveSubscription': 'No active subscription.',
    'dash.viewPricing': 'View pricing',
    'dash.noDevices': 'No registered devices yet. Activate the app on a terminal to register it.',
    'dash.registeredTerminals': 'registered terminals',
    'dash.activePlans': 'active plans',
    'dash.entitlement': 'entitlement',
    'dash.machine': 'Machine',
    'dash.registered': 'Registered',
    'dash.status': 'Status',
    'dash.tier': 'Tier',
    'dash.key': 'Key',
    'dash.starts': 'Starts',
    'dash.expires': 'Expires',
    'dash.graceUntil': 'Grace until',
    'dash.free': 'Free',
    'dash.email': 'Email',
    'dash.emailVerified': 'Email verified',
    'dash.verified': '✓ Verified',
    'dash.unverified': '○ Unverified',
    'dash.couldNotLoad': 'Could not load the dashboard: ',
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
    tenantRow: tenantRow,
    tenantDetailRows: tenantDetailRows,
    svgBarChart: svgBarChart,
    normalizeStats: normalizeStats,
    startLockoutCountdown: startLockoutCountdown,
    startCountdown: startCountdown,
    stopCountdown: stopCountdown,
    countdownActive: countdownActive,
    setAuthMode: setAuthMode,
    busyWrap: busyWrap,
    fetchFxRate: fetchFxRate,
    mountModal: mountModal,
    fetchWithTimeout: fetchWithTimeout,
    exchangeUrlFrom: exchangeUrlFrom,
    isLockoutActive: isLockoutActive,
    statusLabel: statusLabel,
    createSeqGuard: createSeqGuard,
    t: t,
    STRINGS: STRINGS,
    isAuthDenied: isAuthDenied,
    authDeniedError: authDeniedError,
  };
}));