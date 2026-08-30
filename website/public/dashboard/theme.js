// Theme switcher for the user dashboard (light/dark).
// Runs synchronously in <head> so the correct theme applies before first
// paint. Preference persists in localStorage under the same key as the
// admin dashboard so both share the user's choice.
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

  window.__ozDashboardTheme = {
    get: function () { return root.getAttribute('data-theme') || 'dark'; },
    set: function (theme) {
      apply(theme);
      try { localStorage.setItem(KEY, theme); } catch (e) { /* ignore */ }
    },
    toggle: function () {
      var next = (window.__ozDashboardTheme.get() === 'light') ? 'dark' : 'light';
      window.__ozDashboardTheme.set(next);
      return next;
    },
  };
})();
