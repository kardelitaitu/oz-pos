const API = (window.__OZ_CONFIG__ && window.__OZ_CONFIG__.licenseApiUrl) || 'https://license.ozpos.my.id';
    let currentTab = 'dashboard';

    // ── ═══ MOCK DATA (Phase 1 — replaced by real /stats API when available) ═══ ──
    const MOCK = {
      kpis: { totalUsers:1247, activeUsers:1084, totalSubscribers:386, mrrUsd:4280, arpuUsd:11.09, activeDevices:812, trialToPaidRate:22.4 },
      revenueTrend: [
        {month:'2025-09',usd:1280,idr:0},{month:'2025-10',usd:1560,idr:0},{month:'2025-11',usd:1820,idr:0},{month:'2025-12',usd:2140,idr:0},
        {month:'2026-01',usd:2450,idr:0},{month:'2026-02',usd:2710,idr:0},{month:'2026-03',usd:2980,idr:0},{month:'2026-04',usd:3260,idr:0},
        {month:'2026-05',usd:3550,idr:0},{month:'2026-06',usd:3780,idr:0},{month:'2026-07',usd:4020,idr:0},{month:'2026-08',usd:4280,idr:0}
      ],
      subscriberGrowth: [
        {month:'2025-09',count:112},{month:'2025-10',count:145},{month:'2025-11',count:168},{month:'2025-12',count:189},
        {month:'2026-01',count:215},{month:'2026-02',count:238},{month:'2026-03',count:262},{month:'2026-04',count:289},
        {month:'2026-05',count:312},{month:'2026-06',count:338},{month:'2026-07',count:362},{month:'2026-08',count:386}
      ],
      signupsPerMonth: [
        {month:'2025-09',count:41},{month:'2025-10',count:53},{month:'2025-11',count:47},{month:'2025-12',count:62},
        {month:'2026-01',count:58},{month:'2026-02',count:44},{month:'2026-03',count:71},{month:'2026-04',count:66},
        {month:'2026-05',count:55},{month:'2026-06',count:78},{month:'2026-07',count:83},{month:'2026-08',count:87}
      ],
      churnPerMonth: [
        {month:'2025-09',count:2},{month:'2025-10',count:3},{month:'2025-11',count:5},{month:'2025-12',count:4},
        {month:'2026-01',count:6},{month:'2026-02',count:7},{month:'2026-03',count:5},{month:'2026-04',count:8},
        {month:'2026-05',count:6},{month:'2026-06',count:9},{month:'2026-07',count:7},{month:'2026-08',count:12}
      ],
      tierDistribution: [{tier:'plus',count:210},{tier:'pro',count:128},{tier:'premium',count:38},{tier:'enterprise',count:10}],
      providerSplit: [{provider:'paddle',count:264},{provider:'midtrans',count:122}],
      topSubscribers: [
        {email:'resto@warungmakmur.com',tier:'pro',mrrUsd:9.99,renewal:'2026-09-28',provider:'midtrans'},
        {email:'manager@tokosembako.com',tier:'plus',mrrUsd:4.99,renewal:'2026-10-05',provider:'paddle'},
        {email:'owner@bajubatik.com',tier:'premium',mrrUsd:39.99,renewal:'2026-09-15',provider:'paddle'},
        {email:'chef@restoranenak.com',tier:'pro',mrrUsd:9.99,renewal:'2026-11-01',provider:'midtrans'},
        {email:'admin@minimarket24.com',tier:'enterprise',mrrUsd:99.99,renewal:'2026-12-01',provider:'paddle'}
      ],
      recentSignups: [
        {email:'baru@warungbaru.com',created:'2026-08-28',verified:true,tier:'free'},
        {email:'test@coba.com',created:'2026-08-27',verified:false,tier:'free'},
        {email:'owner@tokoabc.com',created:'2026-08-26',verified:true,tier:'plus'},
        {email:'kasir@restoxyz.com',created:'2026-08-24',verified:true,tier:'free'},
        {email:'admin@grosirmurah.com',created:'2026-08-22',verified:true,tier:'pro'}
      ],
      expiringSoon: [
        {email:'owner@bajubatik.com',tier:'premium',expiresAt:'2026-09-15',daysLeft:17},
        {email:'resto@warungmakmur.com',tier:'pro',expiresAt:'2026-09-28',daysLeft:30}
      ]
    };

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
    function el(tag, cls, text) { const e = document.createElement(tag); if (cls) e.className = cls; if (text !== undefined) e.textContent = text; return e; }
    function fmtIdr(val) { return 'Rp ' + Math.round(val).toLocaleString('id-ID'); }
    function fmtUsd(val) { return '$' + Number(val).toFixed(2); }

    function statusPill(status) {
      const map = { active:['pill-ok'], unused:['pill-muted'], grace_period:['pill-warn'], expired:['pill-bad'], revoked:['pill-bad'], paused:['pill-warn'], free:['pill-muted'], plus:['pill-ok'], pro:['pill-warn'], premium:['pill-ok'], enterprise:['pill-ok'] };
      const cls = (map[status] || ['pill-muted'])[0];
      return el('span', 'pill ' + cls, status || '—');
    }

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
          'If you are the admin, please <a href="/" style="color:var(--accent)">sign in again</a>.</p>' +
          '</div>';
        return null;
      }
      if (!res.ok) throw new Error(path + ' (' + res.status + ')');
      return res.json();
    }

    // ── SVG chart helpers ────────────────────────────────────────────
    function svgChart(id, data, series, opts) {
      const w = 600, h = 180, px = 40, py = 20, pw = w - px, ph = h - py - 20;
      const max = Math.max(...data.map(d => Math.max(...series.map(s => d[s]))));
      const min = 0;
      const rng = max - min || 1;
      const x = (i) => px + (i / (data.length - 1 || 1)) * pw;
      const y = (v) => py + ph - ((v - min) / rng) * ph;
      const colors = { usd: '#147efb', idr: '#22c55e', count: '#147efb', mrr: '#147efb' };
      let paths = '', fills = '';
      series.forEach(s => {
        const pts = data.map((d,i) => `${x(i)},${y(d[s])}`).join(' L ');
        paths += `<path d="M ${pts}" stroke="${colors[s]||'#147efb'}" stroke-width="2" fill="none" class="chart-line"/>`;
        if (opts && opts.area) {
          const base = `${x(0)},${py+ph} L ${pts} L ${x(data.length-1)},${py+ph} Z`;
          fills += `<path d="${base}" fill="${colors[s]||'#147efb'}" opacity=".08"/>`;
        }
      });
      // Y axis labels
      let yLabels = '';
      for (let i = 0; i <= 4; i++) { const v = min + (rng / 4) * i; yLabels += `<text x="${px-5}" y="${y(v)+3}" text-anchor="end" fill="var(--muted)" font-size="10">${opts?.fmt ? opts.fmt(v) : Math.round(v)}</text>`; }
      // X axis labels (every 2nd)
      let xLabels = '';
      data.forEach((d,i) => { if (i % 2 === 0 || i === data.length-1) { xLabels += `<text x="${x(i)}" y="${py+ph+15}" text-anchor="middle" fill="var(--muted)" font-size="9">${d.month.slice(5)}</text>`; } });
      return `<svg viewBox="0 0 ${w} ${h}" style="max-height:180px">${fills}${paths}${yLabels}${xLabels}</svg>`;
    }

    function svgDonut(id, data, labelKey, valueKey, colors) {
      const total = data.reduce((s,d) => s + d[valueKey], 0);
      let acc = 0;
      let slices = '';
      const cx = 80, cy = 80, r = 60;
      const colorList = ['#147efb','#22c55e','#e879f9','#fb923c','#22d3ee','#f59e0b'];
      data.forEach((d,i) => {
        const pct = d[valueKey] / total;
        const ang = pct * 360;
        const start = (acc / 360) * 2 * Math.PI - Math.PI/2;
        const end = ((acc + ang) / 360) * 2 * Math.PI - Math.PI/2;
        const x1 = cx + r * Math.cos(start), y1 = cy + r * Math.sin(start);
        const x2 = cx + r * Math.cos(end), y2 = cy + r * Math.sin(end);
        const large = ang > 180 ? 1 : 0;
        const c = colors && colors[i] ? colors[i] : colorList[i % colorList.length];
        slices += `<path d="M ${cx} ${cy} L ${x1} ${y1} A ${r} ${r} 0 ${large} 1 ${x2} ${y2} Z" fill="${c}" stroke="var(--bg)" stroke-width="2"/>`;
        acc += ang;
      });
      // Legend
      let legend = '';
      data.forEach((d,i) => { const c = colors && colors[i] ? colors[i] : colorList[i % colorList.length]; legend += `<div style="display:flex;align-items:center;gap:.4rem;font-size:.75rem;margin:.15rem 0"><span style="width:8px;height:8px;border-radius:2px;background:${c};flex-shrink:0"></span>${d[labelKey]} <span style="color:var(--muted)">${Math.round(pct*100)}%</span></div>`; });
      return { svg: `<svg viewBox="0 0 160 160" style="max-height:160px">${slices}</svg>`, legend };
    }

    // ── Dashboard tab ────────────────────────────────────────────────
    async function renderDashboard() {
      const c = document.getElementById('content');
      c.innerHTML = '<div class="skeleton" style="height:8rem"></div>';

      // Try real stats API first; fall back to MOCK
      let stats = null;
      try { stats = await api('/api/v1/admin/stats'); } catch { /* fall through */ }
      const m = stats || MOCK;
      const isReal = !!stats;

      // FX rate: prefer the real endpoint's value; otherwise fetch live.
      if (isReal && m.kpis && m.kpis.fxRate) { fxRate = m.kpis.fxRate; fxLive = !!m.kpis.fxLive; fxUpdatedAt = m.kpis.fxUpdatedAt || ''; }
      else { await fetchFxRate(); }
      // Convert all revenue data to IDR (client-side fallback for mock)
      if (!isReal) { m.revenueTrend.forEach(d => d.idr = Math.round(d.usd * fxRate)); }
      const mrrIdr = Math.round(m.kpis.mrrUsd * fxRate);

      c.innerHTML = '';

      // --- Mock/Real badge ---
      const top = el('div', null);
      top.style.cssText = 'display:flex;align-items:center;justify-content:space-between;margin-bottom:.5rem';
      const badgeLabel = isReal ? 'LIVE DATA' : 'MOCK DATA';
      const badgeStyle = isReal ? 'background:rgba(34,197,94,.15);color:#4ade80' : 'background:rgba(245,158,11,.15);color:#fbbf24';
      const badge = el('span', null, badgeLabel);
      badge.style.cssText = 'display:inline-block;padding:.1rem .5rem;border-radius:999px;font-size:.65rem;font-weight:600;' + badgeStyle;
      top.appendChild(badge);
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
      const kpiGrid = el('div', 'kpi-grid');
      kpiGrid.appendChild(kpiC('Total Users', String(m.kpis.totalUsers), `active: ${m.kpis.activeUsers}`));
      kpiGrid.appendChild(kpiC('Total Subscribers', String(m.kpis.totalSubscribers), 'non-free (plus/pro/premium/enterprise)'));
      kpiGrid.appendChild(kpiC('MRR', fmtUsd(m.kpis.mrrUsd), ''));
      kpiGrid.appendChild(kpiC('Monthly Gross (IDR)', fmtIdr(mrrIdr), `≈ $${m.kpis.mrrUsd} × ${fxRate.toLocaleString()}`));
      kpiGrid.appendChild(kpiC('ARPU', fmtUsd(m.kpis.arpuUsd), 'per subscriber'));
      kpiGrid.appendChild(kpiC('Active Devices', String(m.kpis.activeDevices), ''));
      kpiGrid.appendChild(kpiC('Trial → Paid', m.kpis.trialToPaidRate + '%', 'conversion rate'));
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
      const tierRow = el('div', null); tierRow.style.cssText = 'display:flex;align-items:center;gap:1rem';
      const donutDiv = el('div', null); donutDiv.innerHTML = donut.svg;
      tierRow.appendChild(donutDiv);
      const legendDiv = el('div', null); legendDiv.innerHTML = donut.legend;
      tierRow.appendChild(legendDiv);
      tierCard.appendChild(tierRow);
      chartGrid.appendChild(tierCard);

      // Provider split (donut)
      const provCard = el('div', 'chart-card');
      provCard.appendChild(el('h3', null, 'Payment Provider'));
      const donut2 = svgDonut('prov', m.providerSplit, 'provider', 'count', ['#147efb','#22c55e']);
      const provRow = el('div', null); provRow.style.cssText = 'display:flex;align-items:center;gap:1rem';
      const donutDiv2 = el('div', null); donutDiv2.innerHTML = donut2.svg;
      provRow.appendChild(donutDiv2);
      const legendDiv2 = el('div', null); legendDiv2.innerHTML = donut2.legend;
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
                 <text x="${bx + barW * 0.35}" y="165" text-anchor="middle" fill="var(--muted)" font-size="8">${d.month.slice(5)}</text>`;
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
                      <text x="${bx + barW * 0.35}" y="165" text-anchor="middle" fill="var(--muted)" font-size="8">${d.month.slice(5)}</text>`;
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

    function kpiC(label, value, sub) {
      const s = el('div', 'kpi');
      s.appendChild(el('div', 'kpi-label', label));
      s.appendChild(el('div', 'kpi-value', value));
      if (sub) s.appendChild(el('div', 'kpi-sub', sub));
      return s;
    }

    function tableCard(heading, headers, rows) {
      const card = el('div', 'card table-card');
      card.appendChild(el('h2', null, heading));
      if (rows.length === 0) { card.appendChild(el('p', 'empty', 'No data.')); return card; }
      const table = el('table');
      const thead = el('thead');
      const tr = el('tr');
      headers.forEach(h => tr.appendChild(el('th', null, h)));
      thead.appendChild(tr); table.appendChild(thead);
      const tbody = el('tbody');
      rows.forEach(row => {
        const tr2 = el('tr');
        row.forEach(cell => tr2.appendChild(el('td', null, cell)));
        tbody.appendChild(tr2);
      });
      table.appendChild(tbody);
      card.appendChild(table);
      return card;
    }

    // ── Tab switching ──────────────────────────────────────────────
    document.querySelectorAll('.tab').forEach(tab => {
      tab.addEventListener('click', () => {
        document.querySelectorAll('.tab').forEach(t => t.classList.remove('tab-active'));
        tab.classList.add('tab-active');
        currentTab = tab.dataset.tab;
        if (currentTab === 'dashboard') renderDashboard();
        if (currentTab === 'tenants') renderTenants();
        if (currentTab === 'health') renderHealth();
      });
    });

    // ── Tenants list (from ADR #42 Phase 3) ─────────────────────────
    let tenants = [];
    async function renderTenants() {
      const c = document.getElementById('content');
      c.innerHTML = '<div class="card"><p class="empty">Loading tenants…</p></div>';
      try { const data = await api('/api/v1/admin/tenants'); if (!data) return; tenants = data.tenants || []; } catch { c.innerHTML = '<div class="card"><p class="empty">Failed to load tenants.</p></div>'; return; }
      c.innerHTML = '';
      const card = el('div', 'card'); card.appendChild(el('h2', null, `Tenants (${tenants.length})`));
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
    }

    // ── Tenant detail (from ADR #42 Phase 3) ────────────────────────
    async function showTenantDetail(id) {
      const modal = document.getElementById('modal-root');
      modal.innerHTML = '<div class="modal-back"><div class="modal"><h3>Loading…</h3></div></div>';
      try {
        const data = await api('/api/v1/admin/tenants/' + id);
        if (!data) { modal.innerHTML = ''; return; }
        const t = data.tenant || {}, lic = data.license || {}, sub = data.subscription || {}, devices = data.devices || [];
        const m = el('div', 'modal-back'), box = el('div', 'modal');
        box.appendChild(el('h3', null, 'Tenant: ' + (t.email || '')));
        const kv = el('div'); kv.style.cssText = 'display:grid;grid-template-columns:1fr 1fr;gap:.5rem;font-size:.82rem';
        kv.innerHTML = '<span class="muted">Status</span><span style="text-align:right">'+(t.status||'—')+'</span>' +
          '<span class="muted">Email verified</span><span style="text-align:right">'+(t.emailVerified?'✓':'○')+'</span>' +
          '<span class="muted">Created</span><span style="text-align:right">'+(t.created?t.created.slice(0,10):'—')+'</span>' +
          '<span class="muted">License key</span><span style="text-align:right;font-family:monospace;font-size:.75rem">'+(lic.key||'—')+'</span>' +
          '<span class="muted">Tier</span><span style="text-align:right">'+(sub.tierKey||lic.tierKey||'—')+'</span>' +
          '<span class="muted">Subscription status</span><span style="text-align:right">'+(sub.status||'—')+'</span>' +
          '<span class="muted">Expires</span><span style="text-align:right">'+(sub.expiresAt||'—')+'</span>' +
          '<span class="muted">Devices</span><span style="text-align:right">'+devices.length+'</span>';
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
      } catch { modal.innerHTML = '<div class="modal-back"><div class="modal"><p class="empty">Failed to load tenant detail.</p></div></div>'; }
    }

    async function doAction(id, action, label, body) {
      const modal = document.getElementById('modal-root');
      try { await api('/api/v1/admin/tenants/' + id + '/' + action, body); modal.innerHTML = ''; flash(label + ' successfully'); renderTenants(); } catch { flash(label + ' failed'); }
    }

    function upgradePrompt(id, data) {
      const modal = document.getElementById('modal-root'), m = el('div', 'modal-back'), box = el('div', 'modal');
      box.appendChild(el('h3', null, 'Change tier'));
      box.innerHTML += '<p class="small" style="margin-bottom:.6rem">Current tier: ' + ((data.subscription && data.subscription.tierKey) || 'none') + '</p>';
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
      try { const h = await api('/api/v1/admin/health'); if (!h) return;
        c.innerHTML = ''; const card = el('div', 'card'); card.appendChild(el('h2', null, 'System Health'));
        const kv = el('div'); kv.style.cssText = 'display:grid;grid-template-columns:1fr 1fr;gap:.5rem;font-size:.82rem';
        const status = h.status === 'ok' ? '✓ OK' : '✗ Degraded';
        kv.innerHTML = '<span class="muted">Status</span><span style="text-align:right">'+status+'</span>' +
          '<span class="muted">Database</span><span style="text-align:right">'+(h.db_ok?'✓ Connected':'✗ Unreachable')+'</span>' +
          '<span class="muted">SMTP</span><span style="text-align:right">'+(h.smtp_host?'✓ Configured':'— Not configured')+'</span>' +
          '<span class="muted">Version</span><span style="text-align:right">'+(h.version||'—')+'</span>' +
          '<span class="muted">Time</span><span style="text-align:right">'+(h.time||'—')+'</span>';
        card.appendChild(kv); c.appendChild(card);
      } catch { c.innerHTML = '<div class="card"><p class="empty">Failed to load health.</p></div>'; }
    }

    // ── Flash ───────────────────────────────────────────────────────
    function flash(msg) { const f = el('div', 'flash', msg); document.body.appendChild(f); setTimeout(() => f.remove(), 3000); }

    // ── Boot ────────────────────────────────────────────────────────
    document.getElementById('logout-btn').addEventListener('click', async () => {
      try { const t = (await (await fetch('/__oz/session')).json()).token; await fetch(API + '/api/v1/web/logout', { method: 'POST', headers: { Authorization: 'Bearer ' + t } }); } catch {}
      window.location.href = '/';
    });

    renderDashboard();
  
