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
      if (isAuthDenied(res.status)) {
        document.getElementById('content').innerHTML =
          '<div class="card" style="text-align:center;padding:2rem">' +
          '<h2 style="margin:0 0 .5rem;color:var(--bad)">' + t('auth.accessDenied') + '</h2>' +
          '<p class="empty">' + t('auth.signInAgain') + '</p>' +
          '</div>';
        // Throw so callers don't overwrite the access-denied screen with a
        // generic error state (the fetch was fine; auth is the problem).
        throw authDeniedError(path);
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
          '<h2 style="margin:0 0 .5rem;color:var(--bad)">' + t('common.statsUnavailable') + '</h2>' +
          '<p class="empty">' + t('common.statsApiNoResponse') + '</p>' +
          '<button class="btn" id="retry-stats">' + t('common.retry') + '</button>' +
          '</div>';
        const retry = document.getElementById('retry-stats');
        if (retry) { retry.addEventListener('click', renderDashboard); }
        return;
      }
      // B6 fix: normalizeStats guarantees the array/kpis shapes the render
      // below dereferences — previously a partial payload made
      // m.revenueTrend.forEach throw before any chart guard could run.
      const m = normalizeStats(stats);

      // FX rate: prefer the real endpoint's value; otherwise fetch live.
      if (m.kpis.fxRate) { fxRate = m.kpis.fxRate; fxLive = !!m.kpis.fxLive; fxUpdatedAt = m.kpis.fxUpdatedAt || ''; }
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
      if (!fxLive) fx.appendChild(el('span', 'small', t('common.stale')));
      top.appendChild(fx);
      c.appendChild(top);

      // --- KPI grid ---
      const ICONS = {
        users: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" role="img" aria-label="Users"><path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M23 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a4 4 0 0 1 0 7.75"/></svg>',
        subscribers: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" role="img" aria-label="Subscribers"><path d="M22 12h-4l-3 9L9 3l-3 9H2"/></svg>',
        mrr: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" role="img" aria-label="MRR"><line x1="12" y1="1" x2="12" y2="23"/><path d="M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6"/></svg>',
        devices: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" role="img" aria-label="Devices"><rect x="2" y="3" width="20" height="14" rx="2" ry="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/></svg>',
        trend: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" role="img" aria-label="Revenue trend"><polyline points="22 7 13.5 15.5 8.5 10.5 2 17"/><polyline points="16 7 22 7 22 13"/></svg>',
        conversion: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" role="img" aria-label="Conversion"><polyline points="23 6 13.5 15.5 8.5 10.5 1 18"/><polyline points="17 6 23 6 23 12"/></svg>',
        arpu: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" role="img" aria-label="ARPU"><circle cx="12" cy="12" r="10"/><path d="M12 6v6l4 2"/></svg>',
      };
      const kpiGrid = el('div', 'kpi-grid');
      kpiGrid.appendChild(kpiC(t('kpi.totalUsers'), String(m.kpis.totalUsers), `active: ${m.kpis.activeUsers}`, ICONS.users, 'kpi-icon-blue'));
      kpiGrid.appendChild(kpiC(t('kpi.totalSubscribers'), String(m.kpis.totalSubscribers), t('toolbar.nonFree'), ICONS.subscribers, 'kpi-icon-green'));
      kpiGrid.appendChild(kpiC(t('kpi.mrr'), fmtUsd(m.kpis.mrrUsd), '', ICONS.mrr, 'kpi-icon-orange'));
      kpiGrid.appendChild(kpiC(t('kpi.monthlyGrossIdr'), fmtIdr(mrrIdr), `≈ $${m.kpis.mrrUsd} × ${fxRate.toLocaleString()}`, ICONS.trend, 'kpi-icon-cyan'));
      kpiGrid.appendChild(kpiC(t('kpi.arpu'), fmtUsd(m.kpis.arpuUsd), t('toolbar.perSubscriber'), ICONS.arpu, 'kpi-icon-pink'));
      kpiGrid.appendChild(kpiC(t('kpi.activeTerminals'), String(m.kpis.activeDevices), '', ICONS.devices, 'kpi-icon-blue'));
      kpiGrid.appendChild(kpiC(t('kpi.trialToPaid'), m.kpis.trialToPaidRate + '%', t('toolbar.conversionRate'), ICONS.conversion, 'kpi-icon-green'));
      c.appendChild(kpiGrid);

      // --- Charts row ---
      const chartGrid = el('div', 'chart-grid');

      // Revenue trend
      const revCard = el('div', 'chart-card');
      revCard.appendChild(el('h3', null, t('chart.revenueTrendIdr')));
      revCard.innerHTML += svgChart('rev', m.revenueTrend, ['idr'], { area: true, fmt: v => 'Rp' + (v/1000000).toFixed(1) + 'jt' });
      chartGrid.appendChild(revCard);

      // Subscriber growth
      const subCard = el('div', 'chart-card');
      subCard.appendChild(el('h3', null, t('chart.subscriberGrowth')));
      subCard.innerHTML += svgChart('subs', m.subscriberGrowth, ['count'], { area: true });
      chartGrid.appendChild(subCard);

      // Tier distribution (donut)
      const tierCard = el('div', 'chart-card');
      tierCard.appendChild(el('h3', null, t('chart.tierDistribution')));
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
      provCard.appendChild(el('h3', null, t('chart.paymentProvider')));
      const donut2 = svgDonut('prov', m.providerSplit, 'provider', 'count', ['#147efb','#22c55e']);
      const provRow = el('div', 'donut-row');
      const donutDiv2 = el('div', 'donut-chart'); donutDiv2.innerHTML = donut2.svg;
      provRow.appendChild(donutDiv2);
      const legendDiv2 = el('div', 'donut-legend'); legendDiv2.innerHTML = donut2.legend;
      provRow.appendChild(legendDiv2);
      provCard.appendChild(provRow);
      chartGrid.appendChild(provCard);

      // Signups per month (bar chart — extracted to admin-utils.svgBarChart)
      const signupCard = el('div', 'chart-card');
      signupCard.appendChild(el('h3', null, t('chart.signupsPerMonth')));
      signupCard.innerHTML += svgBarChart('signups', m.signupsPerMonth, { valueKey: 'count', color: 'var(--accent)' });
      chartGrid.appendChild(signupCard);

      // Churn per month — B3 fix: the server's churnPerMonth rows carry the
      // number in `churn` (count is Go's zero value), so the old inline code
      // reading d.count rendered permanently-zero/NaN bars. Churn also reused
      // the signups barW; each chart now sizes itself.
      const churnCard = el('div', 'chart-card');
      churnCard.appendChild(el('h3', null, t('chart.churnCanceled')));
      churnCard.innerHTML += svgBarChart('churn', m.churnPerMonth, { valueKey: 'churn', color: 'var(--bad)' });
      chartGrid.appendChild(churnCard);

      c.appendChild(chartGrid);

      // --- Tables ---
      // Top subscribers
      if (m.topSubscribers && m.topSubscribers.length > 0) {
        c.appendChild(tableCard(t('table.topSubscribers'), [t('th.email'),t('th.tier'),t('kpi.mrr'),t('th.renewal'),t('th.provider')], m.topSubscribers.map(d => [d.email, d.tier, fmtUsd(d.mrrUsd), d.renewal, d.provider])));
      }
      // Recent signups
      if (m.recentSignups && m.recentSignups.length > 0) {
        c.appendChild(tableCard(t('table.recentSignups'), [t('th.email'),t('th.created'),t('th.emailVerified'),t('th.tier')], m.recentSignups.map(d => [d.email, d.created, d.verified ? '✓' : '○', d.tier])));
      }
      // Expiring soon
      if (m.expiringSoon && m.expiringSoon.length > 0) {
        c.appendChild(tableCard(t('table.expiringSoon'), [t('th.email'),t('th.tier'),t('th.expires'),t('th.daysLeft')], m.expiringSoon.map(d => [d.email, d.tier, d.expiresAt, String(d.daysLeft)])));
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
      c.innerHTML = '<div class="card"><p class="empty">' + t('common.loadingTenants') + '</p></div>';
      let data;
      try {
        const qs = '?page=' + tenantsPage + '&perPage=' + tenantsPerPage +
          (tenantsSearch ? '&search=' + encodeURIComponent(tenantsSearch) : '');
        data = await api('/api/v1/admin/tenants' + qs);
      } catch (err) { if (err && err.authDenied) { return; } c.innerHTML = '<div class="card"><p class="empty">' + t('common.failedToLoadTenants') + '</p></div>'; return; }
      tenants = data.tenants || [];
      tenantsTotal = data.total || 0;

      c.innerHTML = '';

      // ── Search + pagination toolbar ─────────────────────────────
      const toolbar = el('div', 'tenant-toolbar');
      const searchBox = el('input', 'input search-input');
      searchBox.placeholder = t('toolbar.searchPlaceholder');
      searchBox.value = tenantsSearch;
      searchBox.addEventListener('keydown', ev => {
        if (ev.key === 'Enter') { tenantsSearch = searchBox.value.trim(); tenantsPage = 1; renderTenants(); }
      });
      const searchBtn = el('button', 'btn btn-sm', t('toolbar.search'));
      searchBtn.addEventListener('click', () => { tenantsSearch = searchBox.value.trim(); tenantsPage = 1; renderTenants(); });
      const clearBtn = el('button', 'btn btn-sm btn-ghost', t('toolbar.clear'));
      clearBtn.addEventListener('click', () => { tenantsSearch = ''; searchBox.value = ''; tenantsPage = 1; renderTenants(); });
      toolbar.appendChild(searchBox); toolbar.appendChild(searchBtn); toolbar.appendChild(clearBtn);
      const totalLabel = el('span', 'tenant-total', t('toolbar.showing') + tenants.length + t('toolbar.of') + tenantsTotal);
      toolbar.appendChild(totalLabel);
      c.appendChild(toolbar);

      const card = el('div', 'card'); card.appendChild(el('h2', null, t('table.tenants')));
      if (tenants.length === 0) { card.appendChild(el('p', 'empty', t('table.noTenantsMatch'))); c.appendChild(card); return; }
      const table = el('table');
      const thead = el('thead'); const tr = el('tr');
      [t('th.email'),t('th.status'),t('th.license'),t('th.tier'),t('th.created'),''].forEach(h => tr.appendChild(el('th', null, h)));
      thead.appendChild(tr); table.appendChild(thead);
      const tbody = el('tbody');
      // B1 fix: the row builder moved to admin-utils.tenantRow — the old
      // inline callback named its parameter `t`, shadowing the i18n t()
      // helper, so t('tenant.details') threw and the table never rendered.
      tenants.forEach(tenant => tbody.appendChild(tenantRow(tenant, showTenantDetail)));
      table.appendChild(tbody); card.appendChild(table); c.appendChild(card);

      // ── Pagination controls ─────────────────────────────────────
      const totalPages = Math.max(1, Math.ceil(tenantsTotal / tenantsPerPage));
      if (totalPages > 1) {
        const nav = el('div', 'pagination');
        const prev = el('button', 'btn btn-sm btn-ghost', t('toolbar.prev'));
        prev.disabled = tenantsPage <= 1;
        prev.addEventListener('click', () => { if (tenantsPage > 1) { tenantsPage--; renderTenants(); } });
        nav.appendChild(prev);
        const pageInfo = el('span', 'page-info', t('toolbar.page') + tenantsPage + t('toolbar.of') + totalPages);
        nav.appendChild(pageInfo);
        const next = el('button', 'btn btn-sm btn-ghost', t('toolbar.next'));
        next.disabled = tenantsPage >= totalPages;
        next.addEventListener('click', () => { if (tenantsPage < totalPages) { tenantsPage++; renderTenants(); } });
        nav.appendChild(next);
        c.appendChild(nav);
      }
    }

    // ── Tenant detail (from ADR #42 Phase 3) ────────────────────────
    async function showTenantDetail(id) {
      const modal = document.getElementById('modal-root');
      modal.innerHTML = '<div class="modal-back"><div class="modal"><h3>' + t('common.loading') + '</h3></div></div>';
      try {
        const data = await api('/api/v1/admin/tenants/' + id);
        // B2 fix: the old `const t = data.tenant` shadowed the global i18n
        // t(), so every t('…') label below threw TypeError and the modal
        // ALWAYS fell through to "Failed to load tenant detail". The kv
        // mapping now lives in admin-utils.tenantDetailRows (unit-tested).
        const tenant = data.tenant || {};
        const m = el('div', 'modal-back'), box = el('div', 'modal');
        box.setAttribute('role', 'dialog');
        box.setAttribute('aria-modal', 'true');
        box.appendChild(el('h3', null, t('tenant.title') + (tenant.email || '')));
        const kv = el('div'); kv.style.cssText = 'display:grid;grid-template-columns:1fr 1fr;gap:.5rem;font-size:.82rem';
        // Build the key-value grid safely — never innerHTML with API data.
        function addRow(label, val) {
          kv.appendChild(el('span', 'muted', label));
          const vs = el('span', null, val === undefined || val === null ? '—' : String(val));
          vs.style.textAlign = 'right';
          if (label === t('th.licenseKey')) { vs.style.cssText += ';font-family:monospace;font-size:.75rem'; }
          kv.appendChild(vs);
        }
        tenantDetailRows(data).forEach(pair => addRow(pair[0], pair[1]));
        box.appendChild(kv);
        const actions = el('div', null); actions.style.cssText = 'display:flex;gap:.4rem;margin-top:.8rem;flex-wrap:wrap';
        if (tenant.status === 'active') { const revoke = el('button', 'btn btn-sm btn-bad', t('tenant.revoke')); revoke.addEventListener('click', () => doAction(id,'revoke',t('tenant.revoked'))); actions.appendChild(revoke); }
        if (tenant.status !== 'active') { const activate = el('button', 'btn btn-sm btn-ok', t('tenant.activate')); activate.addEventListener('click', () => doAction(id,'activate',t('tenant.activated'))); actions.appendChild(activate); }
        const renew = el('button', 'btn btn-sm', t('tenant.renew365')); renew.addEventListener('click', () => doAction(id,'renew',t('tenant.renewed'),'{"days":365}')); actions.appendChild(renew);
        const upgrade = el('button', 'btn btn-sm btn-warn', t('tenant.upgrade')); upgrade.addEventListener('click', () => upgradePrompt(id,data)); actions.appendChild(upgrade);
        box.appendChild(actions);
        const close = el('button', 'btn btn-ghost', t('tenant.close')); close.style.cssText = 'margin-top:.8rem;width:100%'; close.addEventListener('click', () => { modal.innerHTML = ''; }); box.appendChild(close);
        m.appendChild(box); modal.appendChild(m);
        m.addEventListener('click', e => { if (e.target === m) { modal.innerHTML = ''; } });
        // ESC key closes the modal (a11y #16)
        const escHandler = e => { if (e.key === 'Escape') { modal.innerHTML = ''; document.removeEventListener('keydown', escHandler); } };
        document.addEventListener('keydown', escHandler);
      } catch (err) {
        if (err && err.authDenied) { modal.innerHTML = ''; return; }
        modal.innerHTML = '<div class="modal-back"><div class="modal"><p class="empty">' + t('common.failedToLoadTenantDetail') + '</p></div></div>';
      }
    }

    async function doAction(id, action, label, body) {
      const modal = document.getElementById('modal-root');
      try { await api('/api/v1/admin/tenants/' + id + '/' + action, body); modal.innerHTML = ''; flash(label + t('common.successfully')); renderTenants(); } catch { flash(label + t('common.failed')); }
    }

    function upgradePrompt(id, data) {
      const modal = document.getElementById('modal-root'), m = el('div', 'modal-back'), box = el('div', 'modal');
      box.setAttribute('role', 'dialog');
      box.setAttribute('aria-modal', 'true');
      box.appendChild(el('h3', null, t('tenant.changeTier')));
      const p = el('p', 'small'); p.style.marginBottom = '.6rem'; p.textContent = t('tenant.currentTier') + ((data.subscription && data.subscription.tierKey) || 'none');
      box.appendChild(p);
      const select = el('select', 'input'); ['plus','pro','premium','enterprise'].forEach(tier => { const opt = el('option', null, tier); if (tier === (data.subscription && data.subscription.tierKey)) opt.selected = true; select.appendChild(opt); });
      box.appendChild(select);
      const reason = el('input', 'input'); reason.placeholder = t('tenant.reasonOverride'); reason.style.cssText = 'margin-top:.5rem;' + reason.style.cssText; box.appendChild(reason);
      const act = el('div', 'modal-actions');
      const cancel = el('button', 'btn btn-ghost', t('tenant.cancel')); cancel.addEventListener('click', () => { modal.innerHTML = ''; }); act.appendChild(cancel);
      const save = el('button', 'btn', t('tenant.save')); save.addEventListener('click', async () => { await doAction(id,'tier-override',t('tenant.tierChanged'),JSON.stringify({tier_key:select.value,reason:reason.value||'admin override'})); }); act.appendChild(save);
      box.appendChild(act); m.appendChild(box); modal.appendChild(m);
      m.addEventListener('click', e => { if (e.target === m) { modal.innerHTML = ''; } });
      // ESC key closes the modal (a11y #16)
      const escHandler = e => { if (e.key === 'Escape') { modal.innerHTML = ''; document.removeEventListener('keydown', escHandler); } };
      document.addEventListener('keydown', escHandler);
    }

    // ── Health tab ──────────────────────────────────────────────────
    async function renderHealth() {
      const c = document.getElementById('content'); c.innerHTML = '<div class="card"><p class="empty">' + t('common.loading') + '</p></div>';
      try { const h = await api('/api/v1/admin/health');
        c.innerHTML = ''; const card = el('div', 'card'); card.appendChild(el('h2', null, t('health.title')));
        const kv = el('div'); kv.style.cssText = 'display:grid;grid-template-columns:1fr 1fr;gap:.5rem;font-size:.82rem';
        const status = h.status === 'ok' ? t('health.ok') : t('health.degraded');
        kv.innerHTML = '<span class="muted">'+t('health.status')+'</span><span style="text-align:right">'+escapeHtml(status)+'</span>' +
          '<span class="muted">'+t('health.database')+'</span><span style="text-align:right">'+(h.db_ok?t('health.connected'):t('health.unreachable'))+'</span>' +
          '<span class="muted">'+t('health.smtp')+'</span><span style="text-align:right">'+(h.smtp_host?t('health.configured'):t('health.notConfigured'))+'</span>' +
          '<span class="muted">'+t('health.version')+'</span><span style="text-align:right">'+escapeHtml(h.version||'—')+'</span>' +
          '<span class="muted">'+t('health.time')+'</span><span style="text-align:right">'+escapeHtml(h.time||'—')+'</span>';
        card.appendChild(kv); c.appendChild(card);
      } catch (err) { if (err && err.authDenied) { return; } c.innerHTML = '<div class="card"><p class="empty">' + t('common.failedToLoadHealth') + '</p></div>'; }
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
  
