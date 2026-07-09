import { useState, useEffect, useCallback, type ReactNode } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import RoleBadge from './RoleBadge';
import ThemeToggle from './ThemeToggle';
import UpdateBanner from './UpdateBanner';
import StoreSwitcher from '@/components/StoreSwitcher';
import { GatewayStatusBadge } from '@/components/GatewayStatusBadge';
import { useGatewayStatus } from '@/hooks/useGatewayStatus';
import { useBrand } from '@/contexts/BrandContext';
import { useWorkspaceNav } from '@/hooks/useWorkspaceNav';

import { getNavItems, SECTION_LABELS, type SectionName } from '@/platform/ui/menu-registry';
import './AppLayout.css';

// ── Route type ──────────────────────────────────────────────────────

export type AppRoute = string;

// ── Section ordering ─────────────────────────────────────────────────

const SECTION_ORDER: SectionName[] = [
  'operations',
  'sales',
  'products',
  'finance',
  'customers',
  'reports',
  'inventory',
  'management',
  'settings',
  'dev',
];

function groupBySection<T extends { section?: SectionName }>(items: T[]): { section: SectionName; items: T[] }[] {
  const map = new Map<SectionName, T[]>();
  const seen = new Set<SectionName>();
  for (const item of items) {
    const s = item.section ?? 'management';
    if (!map.has(s)) {
      map.set(s, []);
      seen.add(s);
    }
    map.get(s)!.push(item);
  }
  return SECTION_ORDER.filter((s) => seen.has(s)).map((s) => ({ section: s, items: map.get(s)! }));
}

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
const ADMIN_ROUTES = new Set(['settings', 'features', 'data-management']);

export default function AppLayout({ route, onNavigate, children, enabledFeatures, userRole }: AppLayoutProps) {
  const { l10n } = useLocalization();
  const { settings: brandSettings } = useBrand();
  const showTopbar = !ADMIN_ROUTES.has(route);
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

  // ── Section collapse state (per-section, persisted) ─────────
  const [collapsedSections, setCollapsedSections] = useState<Set<string>>(() => {
    try {
      const raw = localStorage.getItem('app-sidebar-sections');
      return new Set<string>(raw ? JSON.parse(raw) : []);
    } catch { return new Set<string>(); }
  });

  useEffect(() => {
    localStorage.setItem('app-sidebar-sections', JSON.stringify([...collapsedSections]));
  }, [collapsedSections]);

  const toggleSection = useCallback((section: string) => {
    setCollapsedSections((prev) => {
      const next = new Set(prev);
      if (next.has(section)) next.delete(section);
      else next.add(section);
      return next;
    });
  }, []);

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
          {groupBySection(navItems).map(({ section, items }) => {
            const sectionI18nKey = SECTION_LABELS[section];
            const isCollapsed = collapsedSections.has(section);
            return (
              <div key={section} className="app-sidebar-section">
                <button
                  type="button"
                  className="app-sidebar-section-header"
                  onClick={() => toggleSection(section)}
                  aria-expanded={!isCollapsed}
                >
                  <Localized id={sectionI18nKey}>
                    <span className="app-sidebar-section-label">{section}</span>
                  </Localized>
                  <svg
                    className={`app-sidebar-chevron${isCollapsed ? ' collapsed' : ''}`}
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    width="14"
                    height="14"
                    aria-hidden="true"
                  >
                    <polyline points="9 18 15 12 9 6" />
                  </svg>
                </button>
                {!isCollapsed && (
                  <div className="app-sidebar-section-items">
                    {items.map((item) => (
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
                  </div>
                )}
              </div>
            );
          })}
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
        {showTopbar && (
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
        )}
        <UpdateBanner />
        <div className="app-content-inner" key={route}>
          {children}
        </div>
      </main>
    </div>
  );
}
