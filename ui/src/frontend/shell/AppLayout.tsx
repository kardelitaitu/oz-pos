import { type ReactNode } from 'react';
import RoleBadge from './RoleBadge';
import ThemeToggle from './ThemeToggle';
import UpdateBanner from './UpdateBanner';
import { getNavItems } from '@/platform/ui/menu-registry';
import './AppLayout.css';

// ── Route type ──────────────────────────────────────────────────────

export type AppRoute = string;

// ── Props ───────────────────────────────────────────────────────────

export interface AppLayoutProps {
  /** Current active route. */
  route: AppRoute;
  /** Called when the user clicks a navigation item. */
  onNavigate: (route: AppRoute) => void;
  /** Content to render in the main area. */
  children: ReactNode;
  /** Set of enabled feature keys. If omitted, all nav items are shown. */
  enabledFeatures?: Set<string>;
  /** Current user role for role-based nav filtering. */
  userRole?: string;
}

// ── Component ──────────────────────────────────────────────────────

/**
 * Application shell with a sidebar navigation and content area.
 *
 * The sidebar shows the OZ-POS logo, navigation items from the
 * menu-registry, and a theme toggle at the bottom. Nav items that
 * require a disabled feature are hidden.
 *
 * Nav items are registered by feature pages in App.tsx via
 * `registerNavItem()`. The sidebar renders them dynamically
 * instead of using a hardcoded list.
 */
export default function AppLayout({ route, onNavigate, children, enabledFeatures, userRole }: AppLayoutProps) {
  const navItems = getNavItems(enabledFeatures, userRole);

  return (
    <div className="app-layout">
      {/* ── Sidebar ──────────────────────────────── */}
      <aside className="app-sidebar" aria-label="Main navigation">
        <div className="app-sidebar-header">
          <span className="app-sidebar-logo">OZ-POS</span>
        </div>
        <div className="app-sidebar-user">
          <RoleBadge />
        </div>

        <nav className="app-sidebar-nav">
          <span className="app-sidebar-section-label">App</span>

          {navItems.map((item) => (
            <button
              key={item.route}
              type="button"
              className={
                route === item.route
                  ? 'app-nav-item app-nav-item--active'
                  : 'app-nav-item'
              }
              onClick={() => onNavigate(item.route)}
              aria-current={route === item.route ? 'page' : undefined}
              aria-label={item.label}
            >
              {item.icon && (
                <span className="app-nav-icon">{item.icon}</span>
              )}
              {item.label}
            </button>
          ))}
        </nav>

        <div className="app-sidebar-footer">
          <span style={{ fontSize: 'var(--text-xs)', color: 'var(--color-fg-tertiary)' }}>
            v0.0.1
          </span>
          <ThemeToggle />
        </div>
      </aside>

      {/* ── Content area ─────────────────────────── */}
      <main className="app-content">
        <UpdateBanner />
        <div className="app-content-inner" key={route}>
          {children}
        </div>
      </main>
    </div>
  );
}
