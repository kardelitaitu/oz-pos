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

  return {
    el: el,
    escapeHtml: escapeHtml,
    fmtIdr: fmtIdr,
    fmtUsd: fmtUsd,
    statusPill: statusPill,
    svgChart: svgChart,
    svgDonut: svgDonut,
  };
}));