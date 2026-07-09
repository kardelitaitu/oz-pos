import { useState, useEffect, type ReactNode } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import RoleBadge from './RoleBadge';
import ThemeToggle from './ThemeToggle';
import UpdateBanner from './UpdateBanner';
import StoreSwitcher from '@/components/StoreSwitcher';
import { GatewayStatusBadge } from '@/components/GatewayStatusBadge';
import { useGatewayStatus } from '@/hooks/useGatewayStatus';
import { useBrand } from '@/contexts/BrandContext';
import { useWorkspaceNav } from '@/hooks/useWorkspaceNav';

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
  const { l10n } = useLocalization();
  const { settings: brandSettings } = useBrand();
  const navItems = getNavItems(enabledFeatures, userRole);
  const stripeStatus = useGatewayStatus();
  const { goToWorkspacePicker } = useWorkspaceNav();

  // ── Sidebar collapse state (persisted to localStorage) ─────
  const [sidebarCollapsed, setSidebarCollapsed] = useState(() => {
    return localStorage.getItem('app-sidebar-collapsed') === 'true';
  });

  useEffect(() => {
    localStorage.setItem('app-sidebar-collapsed', String(sidebarCollapsed));
  }, [sidebarCollapsed]);

  const toggleSidebar = () => setSidebarCollapsed((prev) => !prev);

  // Set document title to the brand store name (fallback to 'OZ-POS').
  useEffect(() => {
    document.title = brandSettings.store_name
      ? `${brandSettings.store_name} — OZ-POS`
      : 'OZ-POS';
  }, [brandSettings.store_name]);

  return (
    <div className="app-layout">
      {/* ── Sidebar ──────────────────────────────── */}
      <aside className={`app-sidebar${sidebarCollapsed ? ' collapsed' : ''}`} aria-label={l10n.getString('nav-main-aria')}>
        <div className="app-sidebar-header">
          {brandSettings.logo_path ? (
            <img
              className="app-sidebar-logo-img"
              src={`file://${brandSettings.logo_path}`}
              alt=""
            />
          ) : null}
          <span className="app-sidebar-logo">
            {brandSettings.store_name || 'OZ-POS'}
          </span>
        </div>
        <div className="app-sidebar-user">
          <RoleBadge />
        </div>
        <div className="app-sidebar-gateway">
          <GatewayStatusBadge
            gatewayName="Stripe"
            isConfigured={stripeStatus.configured}
            isOnline={stripeStatus.online}
          />
        </div>

        <nav className="app-sidebar-nav">
          <Localized id="nav-section-app"><span className="app-sidebar-section-label">App</span></Localized>

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
              aria-label={l10n.getString(item.i18nKey ?? item.label) ?? item.label}
            >
              {item.icon && (
                <span className="app-nav-icon">{item.icon}</span>
              )}
              <Localized id={item.i18nKey ?? item.label}><span>{item.label}</span></Localized>
            </button>
          ))}
        </nav>

        <div className="app-sidebar-footer">
          <span style={{ fontSize: 'var(--text-xs)', color: 'var(--color-fg-tertiary)' }}>
            v0.0.3
          </span>
          <button
            type="button"
            className="app-sidebar-workspace-btn"
            onClick={goToWorkspacePicker}
            aria-label="Switch workspace"
          >
            <Localized id="nav-switch-workspace">
              <span>Switch Workspace</span>
            </Localized>
          </button>
          <ThemeToggle />
        </div>
      </aside>

      {/* ── Content area ─────────────────────────── */}
      <main className="app-content">
        <div className="app-topbar" role="banner">
          <div className="app-topbar-left">
            <button
              type="button"
              className="sidebar-toggle"
              onClick={toggleSidebar}
              aria-label={l10n.getString(sidebarCollapsed ? 'nav-sidebar-expand' : 'nav-sidebar-collapse')}
              aria-expanded={!sidebarCollapsed}
            >
              {sidebarCollapsed ? (
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="20" height="20" aria-hidden="true">
                  <line x1="3" y1="12" x2="21" y2="12" />
                  <line x1="3" y1="6" x2="21" y2="6" />
                  <line x1="3" y1="18" x2="21" y2="18" />
                </svg>
              ) : (
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="20" height="20" aria-hidden="true">
                  <line x1="18" y1="6" x2="6" y2="18" />
                  <line x1="6" y1="6" x2="18" y2="18" />
                </svg>
              )}
            </button>
          </div>
          <div className="app-topbar-right">
            <StoreSwitcher />
          </div>
        </div>
        <UpdateBanner />
        <div className="app-content-inner" key={route}>
          {children}
        </div>
      </main>
    </div>
  );
}
