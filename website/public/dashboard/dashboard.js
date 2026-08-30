// OZ-POS user dashboard (dashboard.ozpos.my.id) — corporate UX
// following design-language.html: semantic stat cards, card anatomy,
// pills for status, mono for license keys, 4px spacing ladder.

// JWT lives in an httpOnly cookie; fetch it from the same-origin
// /__oz/session endpoint so we can call the license API with Bearer.
// Use the relative API path when on a subdomain so requests flow through
// the Worker /api/v1/ proxy (no cross-origin CORS needed).
const isSubdomain = window.location.hostname.endsWith('ozpos.my.id');
const API = isSubdomain
  ? ''
  : ((window.__OZ_CONFIG__ && window.__OZ_CONFIG__.licenseApiUrl) || 'https://license.ozpos.my.id');

function el(tag, cls, text) {
  const e = document.createElement(tag);
  if (cls) e.className = cls;
  if (text !== undefined) e.textContent = text;
  return e;
}

// Icon set (inline SVG, design-language stroke style).
const ICONS = {
  devices: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="3" width="20" height="14" rx="2" ry="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/></svg>',
  subs: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="1" y="4" width="22" height="16" rx="2" ry="2"/><line x1="1" y1="10" x2="23" y2="10"/></svg>',
  stores: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 9l1.5-5h15L21 9"/><path d="M4 9h16v12H4z"/><path d="M9 21V14h6v7"/></svg>',
  pos: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="5" width="20" height="14" rx="2"/><line x1="2" y1="10" x2="22" y2="10"/><line x1="6" y1="15" x2="10" y2="15"/></svg>',
  user: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"/><circle cx="12" cy="7" r="4"/></svg>',
  key: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4"/></svg>',
  card: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="1" y="4" width="22" height="16" rx="2" ry="2"/><line x1="1" y1="10" x2="23" y2="10"/></svg>',
  check: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>',
};

function statusPill(status) {
  const map = {
    active: ['pill-ok'],
    unused: ['pill-muted'],
    grace_period: ['pill-warn'],
    expired: ['pill-bad'],
    revoked: ['pill-bad'],
    paused: ['pill-warn'],
    suspended: ['pill-bad'],
    free: ['pill-muted'],
    plus: ['pill-ok'],
    pro: ['pill-info'],
    premium: ['pill-warn'],
    enterprise: ['pill-warn'],
  };
  const cls = (map[status] || ['pill-muted'])[0];
  return el('span', 'pill ' + cls, status || '—');
}

async function api(path) {
  let token = null;
  try {
    const sessionRes = await fetch('/__oz/session');
    if (!sessionRes.ok) { window.location.href = '/__oz/logout'; return null; }
    const sessionData = await sessionRes.json();
    token = sessionData.token;
  } catch {
    window.location.href = '/__oz/logout';
    return null;
  }
  if (!token) { window.location.href = '/__oz/logout'; return null; }
  const res = await fetch(API + path, { headers: { Authorization: 'Bearer ' + token } });
  if (res.status === 401) { window.location.href = '/__oz/logout'; return null; }
  if (!res.ok) throw new Error(path + ' failed (' + res.status + ')');
  return res.json();
}

// ── Stat cards (semantic tint + icon) ────────────────────────────
function stat(label, value, sub, icon, variant) {
  const s = el('div', 'stat ' + (variant || 'stat--primary'));
  const ic = el('div', 'stat-icon');
  ic.innerHTML = icon || ICONS.check;
  s.appendChild(ic);
  const body = el('div', 'stat-body');
  body.appendChild(el('div', 'stat-label', label));
  body.appendChild(el('div', 'stat-value', value));
  if (sub) body.appendChild(el('div', 'stat-sub', sub));
  s.appendChild(body);
  return s;
}

function renderUsage(usage) {
  const c = document.getElementById('content');
  const g = el('div', 'stat-grid');
  // Subscription (tier) | Max Stores | Max Terminal | Max KDS
  const tier = usage.tier_key || (usage.subscription_count > 0 ? 'active' : 'free');
  g.appendChild(stat(t('dash.subscription'), tier === 'free' ? t('dash.free') : tier, t('dash.currentPlan'), ICONS.card, 'stat--primary'));
  const maxStores = usage.max_stores ?? 0;
  const maxTerm = usage.max_pos_instances ?? 0;
  const maxKds = usage.max_kds ?? 0;
  const dev = usage.device_count ?? 0;
  g.appendChild(stat(t('dash.maxStores'), fmtUsedMax(0, maxStores), t('dash.stores'), ICONS.stores, 'stat--warning'));
  g.appendChild(stat(t('dash.maxTerminal'), fmtUsedMax(dev, maxTerm), t('dash.registers'), ICONS.pos, 'stat--info'));
  g.appendChild(stat(t('dash.maxKds'), fmtUsedMax(0, maxKds), t('dash.screens'), ICONS.devices, 'stat--success'));
  c.insertBefore(g, c.firstChild);
}

// fmtUsedMax renders "used / max" (or "∞" for the unlimited sentinel).
function fmtUsedMax(used, max) {
  if (max >= 999) return used + ' / ∞';
  return used + ' / ' + max;
}

// ── Cards ────────────────────────────────────────────────────────
function renderMe(me) {
  const c = document.getElementById('content');
  const tenant = me.tenant || {};
  const lic = me.license;
  const sub = me.subscription;

  // Page header
  const title = document.getElementById('page-title');
  const subtitle = document.getElementById('page-sub');
  if (title) title.textContent = tenant.email ? tenant.email : t('dash.dashboard');
  if (subtitle) subtitle.textContent = t('dash.yourAccount');

  // Account card
  const profile = el('div', 'card');
  profile.appendChild(el('h2', null, t('dash.account')));
  const kv = el('dl', 'kv');
  kv.appendChild(el('dt', null, t('dash.email'))); kv.appendChild(el('dd', null, tenant.email || '—'));
  kv.appendChild(el('dt', null, t('dash.status')));
  const dd = el('dd'); dd.appendChild(statusPill(tenant.status)); kv.appendChild(dd);
  kv.appendChild(el('dt', null, t('dash.emailVerified')));
  const ddV = el('dd');
  const verified = el('span', 'pill ' + (tenant.emailVerified ? 'pill-ok' : 'pill-warn'), tenant.emailVerified ? t('dash.verified') : t('dash.unverified'));
  ddV.appendChild(verified); kv.appendChild(ddV);
  profile.appendChild(kv);
  c.appendChild(profile);

  // License card
  if (lic) {
    const card = el('div', 'card');
    card.appendChild(el('h2', null, t('dash.license')));
    const kv2 = el('dl', 'kv');
    kv2.appendChild(el('dt', null, t('dash.key')));
    const ddKey = el('dd');
    const keyRow = el('div'); keyRow.style.cssText = 'display:flex;align-items:center;gap:.5rem;flex-wrap:wrap';
    const keySpan = el('span', 'copykey', lic.key || '—');
    keyRow.appendChild(keySpan);
    const copyBtn = el('button', 'btn btn--ghost btn--sm', t('dash.copy'));
    copyBtn.addEventListener('click', () => {
      navigator.clipboard.writeText(lic.key || '').then(() => { copyBtn.textContent = t('dash.copied'); setTimeout(() => { copyBtn.textContent = t('dash.copy'); }, 1500); });
    });
    keyRow.appendChild(copyBtn);
    ddKey.appendChild(keyRow);
    kv2.appendChild(ddKey);
    kv2.appendChild(el('dt', null, t('dash.tier')));
    const ddTier = el('dd'); ddTier.appendChild(statusPill(lic.tierKey || '—')); kv2.appendChild(ddTier);
    kv2.appendChild(el('dt', null, t('dash.status')));
    const ddStatus = el('dd'); ddStatus.appendChild(statusPill(lic.status)); kv2.appendChild(ddStatus);
    kv2.appendChild(el('dt', null, t('dash.expires'))); kv2.appendChild(el('dd', null, lic.expiresAt || '—'));
    card.appendChild(kv2);
    c.appendChild(card);
  }

  // Subscription card
  const subCard = el('div', 'card');
  subCard.appendChild(el('h2', null, t('dash.subscription')));
  if (sub) {
    const kv3 = el('dl', 'kv');
    kv3.appendChild(el('dt', null, t('dash.tier')));
    const ddT = el('dd'); ddT.appendChild(statusPill(sub.tierKey || '—')); kv3.appendChild(ddT);
    kv3.appendChild(el('dt', null, t('dash.status')));
    const ddS = el('dd'); ddS.appendChild(statusPill(sub.status)); kv3.appendChild(ddS);
    kv3.appendChild(el('dt', null, t('dash.starts'))); kv3.appendChild(el('dd', null, sub.startsAt || '—'));
    kv3.appendChild(el('dt', null, t('dash.expires'))); kv3.appendChild(el('dd', null, sub.expiresAt || '—'));
    if (sub.graceUntil) { kv3.appendChild(el('dt', null, t('dash.graceUntil'))); kv3.appendChild(el('dd', null, sub.graceUntil)); }
    subCard.appendChild(kv3);
  } else {
    subCard.appendChild(el('p', 'empty', t('dash.noActiveSubscription')));
    const a = el('a'); a.href = '/pricing'; a.className = 'btn'; a.textContent = t('dash.viewPricing'); a.style.textDecoration = 'none';
    subCard.appendChild(a);
  }
  c.appendChild(subCard);
}

function renderDevices(devices) {
  const c = document.getElementById('content');
  const card = el('div', 'card table-card');
  card.appendChild(el('h2', null, t('dash.devices')));
  if (!devices || devices.length === 0) {
    card.appendChild(el('p', 'empty', t('dash.noDevices')));
  } else {
    const table = el('table');
    const thead = el('thead');
    const tr = el('tr');
    [t('dash.machine'), t('dash.registered'), t('dash.status')].forEach(h => tr.appendChild(el('th', null, h)));
    thead.appendChild(tr); table.appendChild(thead);
    const tbody = el('tbody');
    devices.forEach(d => {
      const row = el('tr');
      row.appendChild(el('td', null, d.machine_id || '—'));
      row.appendChild(el('td', null, d.created || '—'));
      const td = el('td');
      td.appendChild(d.revoked_at ? statusPill('revoked') : statusPill('active'));
      row.appendChild(td);
      tbody.appendChild(row);
    });
    table.appendChild(tbody);
    card.appendChild(table);
  }
  c.appendChild(card);
}

// ── Boot ────────────────────────────────────────────────────────
(async () => {
  // Theme toggle — theme.js sets data-theme on <html> and exposes
  // window.__ozDashboardTheme.{get,set,toggle}; icons flip via CSS.
  const themeToggle = document.getElementById('theme-toggle');
  if (themeToggle) {
    themeToggle.addEventListener('click', () => {
      if (window.__ozDashboardTheme) { window.__ozDashboardTheme.toggle(); }
    });
  }

  // The httpOnly cookie cannot be deleted by page JS — the Worker must
  // expire it via /__oz/logout.
  document.getElementById('logout-btn').addEventListener('click', () => {
    window.location.href = '/__oz/logout';
  });

  try {
    const [me, usage, dev] = await Promise.all([
      api('/api/v1/web/me'),
      api('/api/v1/web/usage'),
      api('/api/v1/web/devices'),
    ]);
    const contentEl = document.getElementById('content');
    if (contentEl) contentEl.innerHTML = '';
    if (usage) renderUsage(usage);
    if (me) renderMe(me);
    if (dev && dev.devices) renderDevices(dev.devices);
  } catch (err) {
    const contentEl = document.getElementById('content');
    if (contentEl) {
      contentEl.innerHTML = '<div class="card"><p class="empty">' + t('dash.couldNotLoad') + (err && err.message || 'unknown error') + '</p></div>';
    }
  }
})();
