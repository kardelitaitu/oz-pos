// Theme switcher for the admin dashboard (light/dark).
// Runs synchronously in <head> so the correct theme applies before first
// paint (no flash of dark). Loaded as an external file so the strict CSP
// (script-src 'self') allows it. Preference persists in localStorage and
// mirrors to the system-preference default on first visit.
(function () {
  var KEY = 'oz-admin-theme';
  var root = document.documentElement;

  function apply(theme) {
    root.setAttribute('data-theme', theme);
  }

  var saved = null;
  try { saved = localStorage.getItem(KEY); } catch (e) { /* storage unavailable */ }

  if (saved === 'light' || saved === 'dark') {
    apply(saved);
  } else if (window.matchMedia && window.matchMedia('(prefers-color-scheme: light)').matches) {
    apply('light');
  } else {
    apply('dark');
  }

  // Expose the toggle API for admin.js.
  window.__ozAdminTheme = {
    get: function () { return root.getAttribute('data-theme') || 'dark'; },
    set: function (theme) {
      apply(theme);
      try { localStorage.setItem(KEY, theme); } catch (e) { /* ignore */ }
    },
    toggle: function () {
      var next = (window.__ozAdminTheme.get() === 'light') ? 'dark' : 'light';
      window.__ozAdminTheme.set(next);
      return next;
    },
  };
})();
