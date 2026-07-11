import { type ReactNode } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import { getNavItems } from '@/platform/ui/menu-registry';
import './tablet.css';

// ── Props ───────────────────────────────────────────────────────────

/** Props for the tablet-optimised shell layout component. */
export interface TabletAppLayoutProps {
  /** Current active route. */
  route: string;
  /** Called when the user clicks a navigation item. */
  onNavigate: (route: string) => void;
  /** Content to render in the main area. */
  children: ReactNode;
  /** Set of enabled feature keys. If omitted, all nav items are shown. */
  enabledFeatures?: Set<string>;
  /** Current user role for role-based nav filtering. */
  userRole?: string;
  /** ADR #4 Phase 3b: workspace type screens for dynamic tab generation.
   *  When provided, the tab bar shows only screens in this list (in order).
   *  When omitted, falls back to the full menu registry. */
  workspaceScreens?: string[];
}

// ── Component ──────────────────────────────────────────────────────

/**
 * Tablet-optimised application shell.
 *
 * Features:
 * - Bottom tab bar instead of sidebar (thumb-reachable)
 * - Minimum 48px touch targets
 * - Larger typography
 * - Full-screen content with safe-area inset support
 * - Active tab highlighting with accent colour
 */
export default function TabletAppLayout({
  route,
  onNavigate,
  children,
  enabledFeatures,
  userRole,
  workspaceScreens,
}: TabletAppLayoutProps) {
  const { l10n } = useLocalization();
  // ADR #4 Phase 3b: when workspaceScreens is provided, filter nav items
  // to only those matching the workspace type screens. This creates a
  // dynamic, per-instance tab bar instead of a static one.
  const allNavItems = getNavItems(enabledFeatures, userRole);
  const navItems = workspaceScreens && workspaceScreens.length > 0
    ? allNavItems.filter((item) => workspaceScreens.includes(item.route)).slice(0, 7)
    : allNavItems.slice(0, 7); // max 7 tabs for bottom nav

  return (
    <div className="tablet-shell">
      <div className="app-layout">
        {/* ── Main content area ─────────────────────── */}
        <main className="app-content" role="main">
          <div className="app-content-inner" key={route}>
            {children}
          </div>
        </main>

        {/* ── Bottom tab bar ────────────────────────── */}
        <div className="tablet-tab-bar" role="tablist" aria-label={l10n.getString('nav-tablist-aria')}>
          {navItems.map((item) => (
            <button
              key={item.route}
              type="button"
              role="tab"
              className={
                route === item.route
                  ? 'tablet-tab-item tablet-tab-item--active'
                  : 'tablet-tab-item'
              }
              onClick={() => onNavigate(item.route)}
              aria-selected={route === item.route}
              aria-label={l10n.getString(item.i18nKey ?? item.label) ?? item.label}
            >
              {item.icon && (
                <span className="tablet-tab-icon" aria-hidden="true">
                  {item.icon}
                </span>
              )}
              <span className="tablet-tab-label"><Localized id={item.i18nKey ?? item.label}><span>{item.label}</span></Localized></span>
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
