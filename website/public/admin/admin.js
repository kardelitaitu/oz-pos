const API = (window.__OZ_CONFIG__ && window.__OZ_CONFIG__.licenseApiUrl) || 'https://license.ozpos.my.id';
    let currentTab = 'dashboard';

    // ── FX rate (live from open.er-api.com, fallback to 16000) ───
    let fxRate = 16000;
    let fxUpdatedAt = '';
    let fxLive = false;

    async function fetchFxRate() {
      try {
        const r = await fetch('https://open.er-api.com/v6/latest/USD');
        const d = await r.json();
        if (d.rates && d.rates.IDR) { fxRate = d.rates.IDR; fxLive = true; fxUpdatedAt = new Date().toISOString(); }
      } catch { fxLive = false; }
    }

    // ── Helpers ──────────────────────────────────────────────────────
    // el, escapeHtml, fmtIdr, fmtUsd, statusPill, svgChart, svgDonut are
    // defined in admin-utils.js (loaded first) so they're unit-testable.
    // admin-utils.js sets these as globals for backward compatibility.

    async function api(path, body) {
      const token = (await (await fetch('/__oz/session')).json()).token;
      const opts = { headers: { Authorization: 'Bearer ' + token, 'Content-Type': 'application/json' } };
      if (body) { opts.method = 'POST'; opts.body = body; }
      const res = await fetch(API + path, opts);
      if (res.status === 401 || res.status === 403) {
        document.getElementById('content').innerHTML =
          '<div class="card" style="text-align:center;padding:2rem">' +
          '<h2 style="margin:0 0 .5rem;color:var(--bad)">Access denied</h2>' +
          '<p class="empty">Your session is not authorized for the admin panel. ' +
          'If you are the admin, please <a href="/__oz/logout" style="color:var(--accent)">sign in again</a>.</p>' +
          '</div>';
        // Throw so callers don't overwrite the access-denied screen with a
        // generic error state (the fetch was fine; auth is the problem).
        const err = new Error(path + ' (auth denied)');
        err.authDenied = true;
        throw err;
      }
      if (!res.ok) throw new Error(path + ' (' + res.status + ')');
      return res.json();
    }

    // ── SVG chart helpers ────────────────────────────────────────────
    // svgChart, svgDonut are defined in admin-utils.js (loaded first).

    // ── Dashboard tab ────────────────────────────────────────────────
    async function renderDashboard() {
      const c = document.getElementById('content');
      c.innerHTML = '<div class="skeleton" style="height:8rem"></div>';

      // Load real stats; on failure show an error state (no MOCK fallback).
      let stats = null;
      let loadError = null;
      try { stats = await api('/api/v1/admin/stats'); } catch (err) { loadError = err; }
      if (!stats) {
        // api() already rendered an "Access denied" screen for 401/403 —
        // don't overwrite it with a generic error. Only show the retry UI
        // for network / server errors.
        if (loadError && loadError.authDenied) { return; }
        c.innerHTML =
          '<div class="card" style="text-align:center;padding:2rem">' +
          '<h2 style="margin:0 0 .5rem;color:var(--bad)">Stats unavailable</h2>' +
          '<p class="empty">The dashboard API did not respond. Try again.</p>' +
          '<button class="btn" id="retry-stats">Retry</button>' +
          '</div>';
        const retry = document.getElementById('retry-stats');
        if (retry) { retry.addEventListener('click', renderDashboard); }
        return;
      }
      const m = stats;

      // FX rate: prefer the real endpoint's value; otherwise fetch live.
      if (m.kpis && m.kpis.fxRate) { fxRate = m.kpis.fxRate; fxLive = !!m.kpis.fxLive; fxUpdatedAt = m.kpis.fxUpdatedAt || ''; }
      else { await fetchFxRate(); }
      // Convert all revenue data to IDR.
      m.revenueTrend.forEach(d => d.idr = Math.round(d.usd * fxRate));
      const mrrIdr = Math.round(m.kpis.mrrUsd * fxRate);

      c.innerHTML = '';

      // --- FX chip ---
      const top = el('div', null);
      top.style.cssText = 'display:flex;align-items:center;justify-content:space-between;margin-bottom:.5rem';
      const fx = el('div', 'fx-chip');
      const fxDot = el('span', null, fxLive ? '●' : '○');
      fxDot.style.color = fxLive ? 'var(--ok)' : 'var(--warn)';
      fx.appendChild(fxDot);
      fx.appendChild(document.createTextNode(`1 USD = ${fxRate.toLocaleString()} IDR`));
      if (fxUpdatedAt) { fx.appendChild(el('span', 'small', ` (${fxUpdatedAt.slice(11,16)} UTC)`)); }
      if (!fxLive) fx.appendChild(el('span', 'small', ' stale'));
      top.appendChild(fx);
      c.appendChild(top);

      // --- KPI grid ---
      const ICONS = {
        users: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M23 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a4 4 0 0 1 0 7.75"/></svg>',
        subscribers: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 12h-4l-3 9L9 3l-3 9H2"/></svg>',
        mrr: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="1" x2="12" y2="23"/><path d="M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6"/></svg>',
        devices: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="3" width="20" height="14" rx="2" ry="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/></svg>',
        trend: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="22 7 13.5 15.5 8.5 10.5 2 17"/><polyline points="16 7 22 7 22 13"/></svg>',
        conversion: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="23 6 13.5 15.5 8.5 10.5 1 18"/><polyline points="17 6 23 6 23 12"/></svg>',
        arpu: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M12 6v6l4 2"/></svg>',
      };
      const kpiGrid = el('div', 'kpi-grid');
      kpiGrid.appendChild(kpiC('Total Users', String(m.kpis.totalUsers), `active: ${m.kpis.activeUsers}`, ICONS.users, 'kpi-icon-blue'));
      kpiGrid.appendChild(kpiC('Total Subscribers', String(m.kpis.totalSubscribers), 'non-free (plus/pro/premium/enterprise)', ICONS.subscribers, 'kpi-icon-green'));
      kpiGrid.appendChild(kpiC('MRR', fmtUsd(m.kpis.mrrUsd), '', ICONS.mrr, 'kpi-icon-orange'));
      kpiGrid.appendChild(kpiC('Monthly Gross (IDR)', fmtIdr(mrrIdr), `≈ $${m.kpis.mrrUsd} × ${fxRate.toLocaleString()}`, ICONS.trend, 'kpi-icon-cyan'));
      kpiGrid.appendChild(kpiC('ARPU', fmtUsd(m.kpis.arpuUsd), 'per subscriber', ICONS.arpu, 'kpi-icon-pink'));
      kpiGrid.appendChild(kpiC('Active Terminals', String(m.kpis.activeDevices), '', ICONS.devices, 'kpi-icon-blue'));
      kpiGrid.appendChild(kpiC('Trial → Paid', m.kpis.trialToPaidRate + '%', 'conversion rate', ICONS.conversion, 'kpi-icon-green'));
      c.appendChild(kpiGrid);

      // --- Charts row ---
      const chartGrid = el('div', 'chart-grid');

      // Revenue trend
      const revCard = el('div', 'chart-card');
      revCard.appendChild(el('h3', null, 'Revenue Trend (IDR)'));
      revCard.innerHTML += svgChart('rev', m.revenueTrend, ['idr'], { area: true, fmt: v => 'Rp' + (v/1000000).toFixed(1) + 'jt' });
      chartGrid.appendChild(revCard);

      // Subscriber growth
      const subCard = el('div', 'chart-card');
      subCard.appendChild(el('h3', null, 'Subscriber Growth'));
      subCard.innerHTML += svgChart('subs', m.subscriberGrowth, ['count'], { area: true });
      chartGrid.appendChild(subCard);

      // Tier distribution (donut)
      const tierCard = el('div', 'chart-card');
      tierCard.appendChild(el('h3', null, 'Tier Distribution'));
      const donut = svgDonut('tiers', m.tierDistribution, 'tier', 'count');
      const tierRow = el('div', 'donut-row');
      const donutDiv = el('div', 'donut-chart'); donutDiv.innerHTML = donut.svg;
      tierRow.appendChild(donutDiv);
      const legendDiv = el('div', 'donut-legend'); legendDiv.innerHTML = donut.legend;
      tierRow.appendChild(legendDiv);
      tierCard.appendChild(tierRow);
      chartGrid.appendChild(tierCard);

      // Provider split (donut)
      const provCard = el('div', 'chart-card');
      provCard.appendChild(el('h3', null, 'Payment Provider'));
      const donut2 = svgDonut('prov', m.providerSplit, 'provider', 'count', ['#147efb','#22c55e']);
      const provRow = el('div', 'donut-row');
      const donutDiv2 = el('div', 'donut-chart'); donutDiv2.innerHTML = donut2.svg;
      provRow.appendChild(donutDiv2);
      const legendDiv2 = el('div', 'donut-legend'); legendDiv2.innerHTML = donut2.legend;
      provRow.appendChild(legendDiv2);
      provCard.appendChild(provRow);
      chartGrid.appendChild(provCard);

      // Signups per month (bar chart as SVG)
      const signupCard = el('div', 'chart-card');
      signupCard.appendChild(el('h3', null, 'Signups per Month'));
      const maxS = Math.max(...m.signupsPerMonth.map(d => d.count));
      const barW = 420 / m.signupsPerMonth.length;
      let bars = '';
      m.signupsPerMonth.forEach((d,i) => {
        const bh = (d.count / maxS) * 140;
        const bx = 10 + i * (barW + 2);
        bars += `<rect x="${bx}" y="${150 - bh}" width="${barW * 0.7}" height="${bh}" rx="2" fill="var(--accent)" opacity=".8"/>
                 <text x="${bx + barW * 0.35}" y="${150 - bh - 4}" text-anchor="middle" fill="var(--text)" font-size="9">${d.count}</text>
                 <text x="${bx + barW * 0.35}" y="165" text-anchor="middle" fill="var(--muted)" font-size="8">${escapeHtml(d.month.slice(5))}</text>`;
      });
      signupCard.innerHTML += `<svg viewBox="0 0 440 180" style="max-height:180px">${bars}</svg>`;
      chartGrid.appendChild(signupCard);

      // Churn per month
      const churnCard = el('div', 'chart-card');
      churnCard.appendChild(el('h3', null, 'Churn / Canceled'));
      const maxC = Math.max(...m.churnPerMonth.map(d => d.count), 1);
      let churnBars = '';
      m.churnPerMonth.forEach((d,i) => {
        const bh = (d.count / maxC) * 140;
        const bx = 10 + i * (barW + 2);
        churnBars += `<rect x="${bx}" y="${150 - bh}" width="${barW * 0.7}" height="${bh}" rx="2" fill="var(--bad)" opacity=".8"/>
                      <text x="${bx + barW * 0.35}" y="${150 - bh - 4}" text-anchor="middle" fill="var(--text)" font-size="9">${d.count}</text>
                      <text x="${bx + barW * 0.35}" y="165" text-anchor="middle" fill="var(--muted)" font-size="8">${escapeHtml(d.month.slice(5))}</text>`;
      });
      churnCard.innerHTML += `<svg viewBox="0 0 440 180" style="max-height:180px">${churnBars}</svg>`;
      chartGrid.appendChild(churnCard);

      c.appendChild(chartGrid);

      // --- Tables ---
      // Top subscribers
      if (m.topSubscribers && m.topSubscribers.length > 0) {
        c.appendChild(tableCard('Top Subscribers', ['Email','Tier','MRR','Renewal','Provider'], m.topSubscribers.map(d => [d.email, d.tier, fmtUsd(d.mrrUsd), d.renewal, d.provider])));
      }
      // Recent signups
      if (m.recentSignups && m.recentSignups.length > 0) {
        c.appendChild(tableCard('Recent Signups', ['Email','Created','Verified','Tier'], m.recentSignups.map(d => [d.email, d.created, d.verified ? '✓' : '○', d.tier])));
      }
      // Expiring soon
      if (m.expiringSoon && m.expiringSoon.length > 0) {
        c.appendChild(tableCard('Expiring Soon (within 30 days)', ['Email','Tier','Expires','Days Left'], m.expiringSoon.map(d => [d.email, d.tier, d.expiresAt, String(d.daysLeft)])));
      }
    }

// kpiC, tableCard are defined in admin-utils.js (loaded first).

    // ── Tab switching ──────────────────────────────────────────────
    document.querySelectorAll('.nav-btn').forEach(tab => {
      tab.addEventListener('click', () => {
        document.querySelectorAll('.nav-btn').forEach(t => t.classList.remove('nav-active'));
        tab.classList.add('nav-active');
        currentTab = tab.dataset.tab;
        if (currentTab === 'dashboard') renderDashboard();
        if (currentTab === 'tenants') renderTenants();
        if (currentTab === 'health') renderHealth();
      });
    });

    // ── Tenants list (from ADR #42 Phase 3) — search + pagination ─────
    let tenants = [];
    let tenantsPage = 1;
    let tenantsPerPage = 25;
    let tenantsTotal = 0;
    let tenantsSearch = '';

    async function renderTenants() {
      const c = document.getElementById('content');
      c.innerHTML = '<div class="card"><p class="empty">Loading tenants…</p></div>';
      let data;
      try {
        const qs = '?page=' + tenantsPage + '&perPage=' + tenantsPerPage +
          (tenantsSearch ? '&search=' + encodeURIComponent(tenantsSearch) : '');
        data = await api('/api/v1/admin/tenants' + qs);
      } catch (err) { if (err && err.authDenied) { return; } c.innerHTML = '<div class="card"><p class="empty">Failed to load tenants.</p></div>'; return; }
      tenants = data.tenants || [];
      tenantsTotal = data.total || 0;

      c.innerHTML = '';

      // ── Search + pagination toolbar ─────────────────────────────
      const toolbar = el('div', 'tenant-toolbar');
      const searchBox = el('input', 'input search-input');
      searchBox.placeholder = 'Search by email…';
      searchBox.value = tenantsSearch;
      searchBox.addEventListener('keydown', ev => {
        if (ev.key === 'Enter') { tenantsSearch = searchBox.value.trim(); tenantsPage = 1; renderTenants(); }
      });
      const searchBtn = el('button', 'btn btn-sm', 'Search');
      searchBtn.addEventListener('click', () => { tenantsSearch = searchBox.value.trim(); tenantsPage = 1; renderTenants(); });
      const clearBtn = el('button', 'btn btn-sm btn-ghost', 'Clear');
      clearBtn.addEventListener('click', () => { tenantsSearch = ''; searchBox.value = ''; tenantsPage = 1; renderTenants(); });
      toolbar.appendChild(searchBox); toolbar.appendChild(searchBtn); toolbar.appendChild(clearBtn);
      const totalLabel = el('span', 'tenant-total', 'Showing ' + tenants.length + ' of ' + tenantsTotal);
      toolbar.appendChild(totalLabel);
      c.appendChild(toolbar);

      const card = el('div', 'card'); card.appendChild(el('h2', null, 'Tenants'));
      if (tenants.length === 0) { card.appendChild(el('p', 'empty', 'No tenants match.')); c.appendChild(card); return; }
      const table = el('table');
      const thead = el('thead'); const tr = el('tr');
      ['Email','Status','License','Tier','Created',''].forEach(h => tr.appendChild(el('th', null, h)));
      thead.appendChild(tr); table.appendChild(thead);
      const tbody = el('tbody');
      tenants.forEach(t => {
        const row = el('tr');
        row.appendChild(el('td', null, t.email || '—'));
        const td1 = el('td'); td1.appendChild(statusPill(t.status)); row.appendChild(td1);
        row.appendChild(el('td', null, (t.license && t.license.key) || '—'));
        row.appendChild(el('td', null, (t.subscription && t.subscription.tierKey) || '—'));
        row.appendChild(el('td', null, t.created ? t.created.slice(0,10) : '—'));
        const tdAction = el('td'); const btn = el('button', 'btn btn-sm btn-ghost', 'Details');
        btn.addEventListener('click', () => showTenantDetail(t.id)); tdAction.appendChild(btn); row.appendChild(tdAction);
        tbody.appendChild(row);
      });
      table.appendChild(tbody); card.appendChild(table); c.appendChild(card);

      // ── Pagination controls ─────────────────────────────────────
      const totalPages = Math.max(1, Math.ceil(tenantsTotal / tenantsPerPage));
      if (totalPages > 1) {
        const nav = el('div', 'pagination');
        const prev = el('button', 'btn btn-sm btn-ghost', '← Prev');
        prev.disabled = tenantsPage <= 1;
        prev.addEventListener('click', () => { if (tenantsPage > 1) { tenantsPage--; renderTenants(); } });
        nav.appendChild(prev);
        const pageInfo = el('span', 'page-info', 'Page ' + tenantsPage + ' of ' + totalPages);
        nav.appendChild(pageInfo);
        const next = el('button', 'btn btn-sm btn-ghost', 'Next →');
        next.disabled = tenantsPage >= totalPages;
        next.addEventListener('click', () => { if (tenantsPage < totalPages) { tenantsPage++; renderTenants(); } });
        nav.appendChild(next);
        c.appendChild(nav);
      }
    }

    // ── Tenant detail (from ADR #42 Phase 3) ────────────────────────
    async function showTenantDetail(id) {
      const modal = document.getElementById('modal-root');
      modal.innerHTML = '<div class="modal-back"><div class="modal"><h3>Loading…</h3></div></div>';
      try {
        const data = await api('/api/v1/admin/tenants/' + id);
        const t = data.tenant || {}, lic = data.license || {}, sub = data.subscription || {}, devices = data.devices || [];
        const m = el('div', 'modal-back'), box = el('div', 'modal');
        box.appendChild(el('h3', null, 'Tenant: ' + (t.email || '')));
        const kv = el('div'); kv.style.cssText = 'display:grid;grid-template-columns:1fr 1fr;gap:.5rem;font-size:.82rem';
        // Build the key-value grid safely — never innerHTML with API data.
        function addRow(label, val) {
          kv.appendChild(el('span', 'muted', label));
          const vs = el('span', null, val === undefined || val === null ? '—' : String(val));
          vs.style.textAlign = 'right';
          if (label === 'License key') { vs.style.cssText += ';font-family:monospace;font-size:.75rem'; }
          kv.appendChild(vs);
        }
        addRow('Status', t.status);
        addRow('Email verified', t.emailVerified ? '✓' : '○');
        addRow('Created', t.created ? t.created.slice(0,10) : '—');
        addRow('License key', lic.key || '—');
        addRow('Tier', sub.tierKey || lic.tierKey || '—');
        addRow('Subscription status', sub.status || '—');
        addRow('Expires', sub.expiresAt || '—');
        addRow('Devices', devices.length);
        box.appendChild(kv);
        const actions = el('div', null); actions.style.cssText = 'display:flex;gap:.4rem;margin-top:.8rem;flex-wrap:wrap';
        if (t.status === 'active') { const revoke = el('button', 'btn btn-sm btn-bad', 'Revoke'); revoke.addEventListener('click', () => doAction(id,'revoke','Revoked')); actions.appendChild(revoke); }
        if (t.status !== 'active') { const activate = el('button', 'btn btn-sm btn-ok', 'Activate'); activate.addEventListener('click', () => doAction(id,'activate','Activated')); actions.appendChild(activate); }
        const renew = el('button', 'btn btn-sm', 'Renew +365d'); renew.addEventListener('click', () => doAction(id,'renew','Renewed','{"days":365}')); actions.appendChild(renew);
        const upgrade = el('button', 'btn btn-sm btn-warn', 'Upgrade'); upgrade.addEventListener('click', () => upgradePrompt(id,data)); actions.appendChild(upgrade);
        box.appendChild(actions);
        const close = el('button', 'btn btn-ghost', 'Close'); close.style.cssText = 'margin-top:.8rem;width:100%'; close.addEventListener('click', () => { modal.innerHTML = ''; }); box.appendChild(close);
        m.appendChild(box); modal.appendChild(m);
        m.addEventListener('click', e => { if (e.target === m) { modal.innerHTML = ''; } });
      } catch (err) {
        if (err && err.authDenied) { modal.innerHTML = ''; return; }
        modal.innerHTML = '<div class="modal-back"><div class="modal"><p class="empty">Failed to load tenant detail.</p></div></div>';
      }
    }

    async function doAction(id, action, label, body) {
      const modal = document.getElementById('modal-root');
      try { await api('/api/v1/admin/tenants/' + id + '/' + action, body); modal.innerHTML = ''; flash(label + ' successfully'); renderTenants(); } catch { flash(label + ' failed'); }
    }

    function upgradePrompt(id, data) {
      const modal = document.getElementById('modal-root'), m = el('div', 'modal-back'), box = el('div', 'modal');
      box.appendChild(el('h3', null, 'Change tier'));
      const p = el('p', 'small'); p.style.marginBottom = '.6rem'; p.textContent = 'Current tier: ' + ((data.subscription && data.subscription.tierKey) || 'none');
      box.appendChild(p);
      const select = el('select', 'input'); ['plus','pro','premium','enterprise'].forEach(t => { const opt = el('option', null, t); if (t === (data.subscription && data.subscription.tierKey)) opt.selected = true; select.appendChild(opt); });
      box.appendChild(select);
      const reason = el('input', 'input'); reason.placeholder = 'Reason for override (audit)'; reason.style.cssText = 'margin-top:.5rem;' + reason.style.cssText; box.appendChild(reason);
      const act = el('div', 'modal-actions');
      const cancel = el('button', 'btn btn-ghost', 'Cancel'); cancel.addEventListener('click', () => { modal.innerHTML = ''; }); act.appendChild(cancel);
      const save = el('button', 'btn', 'Save'); save.addEventListener('click', async () => { await doAction(id,'tier-override','Tier changed',JSON.stringify({tier_key:select.value,reason:reason.value||'admin override'})); }); act.appendChild(save);
      box.appendChild(act); m.appendChild(box); modal.appendChild(m);
      m.addEventListener('click', e => { if (e.target === m) { modal.innerHTML = ''; } });
    }

    // ── Health tab ──────────────────────────────────────────────────
    async function renderHealth() {
      const c = document.getElementById('content'); c.innerHTML = '<div class="card"><p class="empty">Loading…</p></div>';
      try { const h = await api('/api/v1/admin/health');
        c.innerHTML = ''; const card = el('div', 'card'); card.appendChild(el('h2', null, 'System Health'));
        const kv = el('div'); kv.style.cssText = 'display:grid;grid-template-columns:1fr 1fr;gap:.5rem;font-size:.82rem';
        const status = h.status === 'ok' ? '✓ OK' : '✗ Degraded';
        kv.innerHTML = '<span class="muted">Status</span><span style="text-align:right">'+escapeHtml(status)+'</span>' +
          '<span class="muted">Database</span><span style="text-align:right">'+(h.db_ok?'✓ Connected':'✗ Unreachable')+'</span>' +
          '<span class="muted">SMTP</span><span style="text-align:right">'+(h.smtp_host?'✓ Configured':'— Not configured')+'</span>' +
          '<span class="muted">Version</span><span style="text-align:right">'+escapeHtml(h.version||'—')+'</span>' +
          '<span class="muted">Time</span><span style="text-align:right">'+escapeHtml(h.time||'—')+'</span>';
        card.appendChild(kv); c.appendChild(card);
      } catch (err) { if (err && err.authDenied) { return; } c.innerHTML = '<div class="card"><p class="empty">Failed to load health.</p></div>'; }
    }

    // ── Flash ───────────────────────────────────────────────────────
    function flash(msg) { const f = el('div', 'flash', msg); document.body.appendChild(f); setTimeout(() => f.remove(), 3000); }

    // ── Boot ────────────────────────────────────────────────────────
    document.getElementById('logout-btn').addEventListener('click', () => {
      window.location.href = '/__oz/logout';
    });

    // Theme toggle (light/dark) — theme.js sets data-theme on <html> and
    // exposes window.__ozAdminTheme.{get,set,toggle}; the icons flip via CSS.
    const themeToggle = document.getElementById('theme-toggle');
    if (themeToggle) {
      themeToggle.addEventListener('click', () => {
        if (window.__ozAdminTheme) { window.__ozAdminTheme.toggle(); }
      });
    }

    renderDashboard();
  
