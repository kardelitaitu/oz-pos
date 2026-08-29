    // The JWT lives in an httpOnly cookie; fetch it from the same-origin
    // /__oz/session endpoint so we can call the license API with Bearer.
    const API = (window.__OZ_CONFIG__ && window.__OZ_CONFIG__.licenseApiUrl) || 'https://license.ozpos.my.id';

    function el(tag, cls, text) { const e = document.createElement(tag); if (cls) e.className = cls; if (text !== undefined) e.textContent = text; return e; }

    function statusPill(status) {
      const map = { active:['pill-ok'], unused:['pill-muted'], grace_period:['pill-warn'], expired:['pill-bad'], revoked:['pill-bad'], paused:['pill-warn'] };
      const cls = (map[status] || ['pill-muted'])[0];
      const p = el('span', 'pill ' + cls, status || '—');
      return p;
    }

    async function api(path) {
      const token = (await (await fetch('/__oz/session')).json()).token;
      const res = await fetch(API + path, { headers: { Authorization: 'Bearer ' + token } });
      if (res.status === 401) { window.location.href = '/en/login?redirect=/'; return null; }
      if (!res.ok) throw new Error(path + ' failed (' + res.status + ')');
      return res.json();
    }

    // ── Render ──────────────────────────────────────────────────────
    function renderMe(me) {
      const c = document.getElementById('content');
      c.innerHTML = '';
      const tenant = me.tenant || {};
      const lic = me.license;
      const sub = me.subscription;

      // Profile card
      const profile = el('div', 'card');
      profile.appendChild(el('h2', null, 'Account'));
      const kv = el('dl', 'kv');
      kv.appendChild(el('dt', null, 'Email')); kv.appendChild(el('dd', null, tenant.email || '—'));
      kv.appendChild(el('dt', null, 'Status'));
      const dd = el('dd'); dd.appendChild(statusPill(tenant.status)); kv.appendChild(dd);
      kv.appendChild(el('dt', null, 'Email verified')); kv.appendChild(el('dd', null, tenant.emailVerified ? '✓ Yes' : '○ No'));
      profile.appendChild(kv);
      c.appendChild(profile);

      // License card
      if (lic) {
        const card = el('div', 'card');
        card.appendChild(el('h2', null, 'License'));
        const kv2 = el('dl', 'kv');
        kv2.appendChild(el('dt', null, 'Key'));
        const ddKey = el('dd'); const keySpan = el('span', 'copykey', lic.key || '—'); ddKey.appendChild(keySpan); kv2.appendChild(ddKey);
        kv2.appendChild(el('dt', null, 'Tier')); kv2.appendChild(el('dd', null, lic.tierKey || '—'));
        const ddStatus = el('dd'); ddStatus.appendChild(statusPill(lic.status)); kv2.appendChild(el('dt', null, 'Status')); kv2.appendChild(ddStatus);
        kv2.appendChild(el('dt', null, 'Expires')); kv2.appendChild(el('dd', null, lic.expiresAt || '—'));
        card.appendChild(kv2);
        c.appendChild(card);
      }

      // Subscription card
      const subCard = el('div', 'card');
      subCard.appendChild(el('h2', null, 'Subscription'));
      if (sub) {
        const kv3 = el('dl', 'kv');
        kv3.appendChild(el('dt', null, 'Tier')); kv3.appendChild(el('dd', null, sub.tierKey || '—'));
        const ddS = el('dd'); ddS.appendChild(statusPill(sub.status)); kv3.appendChild(el('dt', null, 'Status')); kv3.appendChild(ddS);
        kv3.appendChild(el('dt', null, 'Starts')); kv3.appendChild(el('dd', null, sub.startsAt || '—'));
        kv3.appendChild(el('dt', null, 'Expires')); kv3.appendChild(el('dd', null, sub.expiresAt || '—'));
        if (sub.graceUntil) { kv3.appendChild(el('dt', null, 'Grace until')); kv3.appendChild(el('dd', null, sub.graceUntil)); }
        subCard.appendChild(kv3);
      } else {
        subCard.appendChild(el('p', 'empty', 'No active subscription.'));
        const a = el('a'); a.href = '/pricing'; a.className = 'btn'; a.textContent = 'View pricing'; a.style.textDecoration = 'none';
        subCard.appendChild(a);
      }
      c.appendChild(subCard);
    }

    function renderUsage(usage) {
      const c = document.getElementById('content');
      const g = el('div', 'grid');
      g.appendChild(stat('Devices', String(usage.device_count ?? 0), 'registered terminals'));
      g.appendChild(stat('Subscriptions', String(usage.subscription_count ?? 0), 'active plans'));
      g.appendChild(stat('Max stores', String(usage.max_stores ?? 0), 'entitlement'));
      g.appendChild(stat('Max POS', String(usage.max_pos_instances ?? 0), 'entitlement'));
      c.insertBefore(g, c.firstChild);
    }

    function stat(label, value, sub) {
      const s = el('div', 'stat');
      s.appendChild(el('div', 'stat-label', label));
      s.appendChild(el('div', 'stat-value', value));
      if (sub) s.appendChild(el('div', 'stat-sub', sub));
      return s;
    }

    function renderDevices(devices) {
      const c = document.getElementById('content');
      const card = el('div', 'card');
      card.appendChild(el('h2', null, 'Devices'));
      if (!devices || devices.length === 0) {
        card.appendChild(el('p', 'empty', 'No registered devices yet. Activate the app on a terminal to register it.'));
      } else {
        const table = el('table');
        const thead = el('thead');
        const tr = el('tr');
        ['Machine', 'Registered', 'Status'].forEach(h => tr.appendChild(el('th', null, h)));
        thead.appendChild(tr); table.appendChild(thead);
        const tbody = el('tbody');
        devices.forEach(d => {
          const row = el('tr');
          row.appendChild(el('td', null, d.machine_id || '—'));
          row.appendChild(el('td', null, d.created || '—'));
          const td = el('td');
          if (d.revoked_at) { td.appendChild(statusPill('revoked')); }
          else { td.appendChild(statusPill('active')); }
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
      document.getElementById('logout-btn').addEventListener('click', async () => {
        try { const t = (await (await fetch('/__oz/session')).json()).token; await fetch(API + '/api/v1/web/logout', { method: 'POST', headers: { Authorization: 'Bearer ' + t } }); } catch {}
        window.location.href = '/';
      });

      try {
        const [me, usage, dev] = await Promise.all([
          api('/api/v1/web/me'),
          api('/api/v1/web/usage'),
          api('/api/v1/web/devices'),
        ]);
        renderUsage(usage);
        renderMe(me);
        renderDevices(dev && dev.devices);
      } catch (err) {
        document.getElementById('content').innerHTML = '<div class="card"><p class="empty">Could not load the dashboard: ' + (err && err.message || 'unknown error') + '</p></div>';
      }
    })();
  
