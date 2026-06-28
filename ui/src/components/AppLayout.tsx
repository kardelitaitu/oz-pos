import { type ReactNode } from 'react';
import ThemeToggle from './ThemeToggle';
import './AppLayout.css';

// ── Route type ──────────────────────────────────────────────────────

export type AppRoute = 'design' | 'sales';

// ── Navigation item definition ──────────────────────────────────────

interface NavItem {
  route: AppRoute;
  label: string;
  icon: ReactNode;
}

const NAV_ITEMS: NavItem[] = [
  {
    route: 'sales',
    label: 'POS Terminal',
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <rect x="2" y="3" width="20" height="14" rx="2" ry="2" />
        <line x1="8" y1="21" x2="16" y2="21" />
        <line x1="12" y1="17" x2="12" y2="21" />
      </svg>
    ),
  },
  {
    route: 'design',
    label: 'Design System',
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <circle cx="12" cy="12" r="3" />
        <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
      </svg>
    ),
  },
];

// ── Props ───────────────────────────────────────────────────────────

export interface AppLayoutProps {
  /** Current active route. */
  route: AppRoute;
  /** Called when the user clicks a navigation item. */
  onNavigate: (route: AppRoute) => void;
  /** Content to render in the main area. */
  children: ReactNode;
}

// ── Component ──────────────────────────────────────────────────────

/**
 * Application shell with a sidebar navigation and content area.
 *
 * The sidebar shows the OZ-POS logo, navigation items with SVG icons,
 * and a theme toggle at the bottom.
 */
export default function AppLayout({ route, onNavigate, children }: AppLayoutProps) {
  return (
    <div className="app-layout">
      {/* ── Sidebar ──────────────────────────────── */}
      <aside className="app-sidebar" aria-label="Main navigation">
        <div className="app-sidebar-header">
          <span className="app-sidebar-logo">OZ-POS</span>
        </div>

        <nav className="app-sidebar-nav">
          <span className="app-sidebar-section-label">App</span>

          {NAV_ITEMS.map((item) => (
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
              <span className="app-nav-icon">{item.icon}</span>
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
        <div className="app-content-inner" key={route}>
          {children}
        </div>
      </main>
    </div>
  );
}
