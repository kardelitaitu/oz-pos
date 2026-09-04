import { useEffect } from 'react';
import { Localized } from '@fluent/react';

/**
 * Branded boot splash — React stage (stage 2) of the two-stage loading
 * screen.
 *
 * Stage 1 is the static markup rendered by `index.html` /
 * `index.tablet.html` (visible from the very first paint, before any
 * JS runs). This component renders the identical visual so the handoff
 * from static HTML to React is seamless: the same logo, spinner ring,
 * and localized status label occupy the same centered layout.
 *
 * The logo is an inline SVG copy of `ui/public/favicon.svg` — an
 * `<img src>` fetch races the first paint on F5 and pops in late, so
 * every splash stage inlines the mark. The gradient id is unique per
 * copy (`-r` for this React stage, `-d`/`-t` in the HTML entries)
 * because inlined SVG gradient ids share one document namespace and
 * the static stage briefly coexists with this one.
 *
 * Rendered by AppShell / TabletAppShell while the boot gate (license +
 * setup restore) is in flight, replacing the previous bare-text
 * "Loading…" state.
 *
 * On mount it removes the static stage-1 node (`#boot-splash`), which
 * sits outside `#root` and is therefore not cleared by React.
 *
 * Keep the markup and inline styles synchronized with both HTML entry
 * files (`.app-splash` rules in their `<head>` style blocks).
 */
export function AppBootSplash() {
  useEffect(() => {
    document.getElementById('boot-splash')?.remove();
  }, []);

  return (
    <div className="app-splash" role="status" aria-live="polite" aria-busy="true">
      <svg className="app-splash__logo" viewBox="0 0 100 100" aria-hidden="true" focusable="false">
        <defs>
          <linearGradient id="oz-splash-bg-r" x1="0%" y1="0%" x2="100%" y2="100%">
            <stop offset="0%" stopColor="#2563EB" />
            <stop offset="100%" stopColor="#6366F1" />
          </linearGradient>
        </defs>
        <rect width="100" height="100" rx="20" fill="url(#oz-splash-bg-r)" />
        <g fill="#FFFFFF" transform="translate(50,48) scale(1.1)">
          <path
            d="M0,-30 C8,-30 16,-28.5 23,-25 C30,-21.5 36,-16.5 40,-10
               C44,-3.5 46,3.5 46,11 C46,18.5 44,25 40,31
               C36,37 30,41 23,44 C16,47 8,48.5 0,48.5
               C-8,48.5 -16,47 -23,44 C-30,41 -36,37 -40,31
               C-44,25 -46,18.5 -46,11 C-46,3.5 -44,-3.5 -40,-10
               C-36,-16.5 -30,-21.5 -23,-25 C-16,-28.5 -8,-30 0,-30Z"
            fillOpacity={0.15}
          />
          <path d="M0,-28 C-6,-28 -12,-26 -17,-22.5 C-22,-19 -26,-14 -28,-8
               C-30,-2 -30,4 -28,10 C-26,16 -22,21 -17,24.5
               C-12,28 -6,30 0,30 C6,30 12,28 17,24.5
               C22,21 26,16 28,10 C30,4 30,-2 28,-8
               C26,-14 22,-19 17,-22.5 C12,-26 6,-28 0,-28Z" />
          <path d="M0,-18 C-4,-18 -7.5,-16.5 -10,-14 C-12.5,-11.5 -14,-8 -14,-4
               C-14,0 -12.5,3.5 -10,6 C-7.5,8.5 -4,10 0,10
               C4,10 7.5,8.5 10,6 C12.5,3.5 14,0 14,-4
               C14,-8 12.5,-11.5 10,-14 C7.5,-16.5 4,-18 0,-18Z" />
        </g>
        <g fill="#FFFFFF" transform="translate(50,75)">
          <rect x="-18" y="0" width="4" height="18" rx="1.5" />
          <rect x="-6" y="0" width="4" height="18" rx="1.5" />
          <rect x="6" y="0" width="4" height="18" rx="1.5" />
          <rect x="18" y="0" width="4" height="18" rx="1.5" />
        </g>
      </svg>
      <span className="app-splash__spinner" aria-hidden="true" />
      <Localized id="shared-loading">
        <span className="app-splash__label">Loading&hellip;</span>
      </Localized>
    </div>
  );
}
