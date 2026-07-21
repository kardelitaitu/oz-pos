import { useEffect, useState, useCallback, useMemo, useRef } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import Tooltip from '@/frontend/shell/Tooltip';

// ── Sidebar nav item type ─────────────────────────────────────────

interface SettingsNavItem {
  key: string;
  label: string;
  icon: React.ReactNode;
}

const NAV_ITEMS: SettingsNavItem[] = [
  {
    key: 'general',
    label: 'General',
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <rect x="3" y="3" width="7" height="7" />
        <rect x="14" y="3" width="7" height="7" />
        <rect x="3" y="14" width="7" height="7" />
        <rect x="14" y="14" width="7" height="7" />
      </svg>
    ),
  },
  {
    key: 'appearance',
    label: 'Appearance',
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <circle cx="12" cy="12" r="3" />
        <path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42" />
      </svg>
    ),
  },
  {
    key: 'receipt',
    label: 'Receipt',
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
        <polyline points="14 2 14 8 20 8" />
        <line x1="16" y1="13" x2="8" y2="13" />
        <line x1="16" y1="17" x2="8" y2="17" />
      </svg>
    ),
  },
  {
    key: 'sync',
    label: 'Cloud Sync',
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
      </svg>
    ),
  },
  {
    key: 'about',
    label: 'About',
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <circle cx="12" cy="12" r="10" />
        <line x1="12" y1="16" x2="12" y2="12" />
        <line x1="12" y1="8" x2="12.01" y2="8" />
      </svg>
    ),
  },
  {
    key: 'features',
    label: 'Features',
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <path d="M13 2 3 14h9l-1 8 10-12h-9z" />
      </svg>
    ),
  },

  {
    key: 'data',
    label: 'Data',
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <ellipse cx="12" cy="5" rx="9" ry="3" />
        <path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3" />
        <path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5" />
      </svg>
    ),
  },
  {
    key: 'staff',
    label: 'Staff',
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" />
        <circle cx="9" cy="7" r="4" />
        <path d="M23 21v-2a4 4 0 0 0-3-3.87" />
        <path d="M16 3.13a4 4 0 0 1 0 7.75" />
      </svg>
    ),
  },
  {
    key: 'terminals',
    label: 'Terminals',
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <rect x="2" y="3" width="20" height="14" rx="2" />
        <line x1="8" y1="21" x2="16" y2="21" />
        <line x1="12" y1="17" x2="12" y2="21" />
        <path d="M7 7l3 3-3 3" />
      </svg>
    ),
  },
  {
    key: 'stores',
    label: 'Stores',
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
        <polyline points="9 22 9 12 15 12 15 22" />
      </svg>
    ),
  },
  {
    key: 'audit',
    label: 'Audit Log',
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
        <polyline points="14 2 14 8 20 8" />
        <line x1="16" y1="13" x2="8" y2="13" />
        <line x1="16" y1="17" x2="8" y2="17" />
      </svg>
    ),
  },
  {
    key: 'offline',
    label: 'Offline Queue',
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
      </svg>
    ),
  },
  {
    key: 'shifts',
    label: 'Shifts',
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <circle cx="12" cy="12" r="10" />
        <polyline points="12 6 12 12 16 14" />
      </svg>
    ),
  },
  {
    key: 'tax',
    label: 'Tax Rates',
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <line x1="4" y1="6" x2="20" y2="6" />
        <line x1="4" y1="12" x2="20" y2="12" />
        <line x1="4" y1="18" x2="20" y2="18" />
        <line x1="8" y1="6" x2="8" y2="18" />
      </svg>
    ),
  },
  {
    key: 'license',
    label: 'License',
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <path d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4" />
      </svg>
    ),
  },
  {
    key: 'exchange',
    label: 'Exchange Rates',
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <path d="M12 1v22M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6" />
        <line x1="12" y1="1" x2="12" y2="23" />
      </svg>
    ),
  },
  {
    key: 'promotions',
    label: 'Promotions',
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" />
      </svg>
    ),
  },
  {
    key: 'email',
    label: 'Email Reports',
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <rect x="2" y="4" width="20" height="16" rx="2" />
        <path d="M22 7l-10 7L2 7" />
      </svg>
    ),
  },
  {
    key: 'topology',
    label: 'Topology',
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <circle cx="6" cy="6" r="3" />
        <circle cx="18" cy="6" r="3" />
        <circle cx="12" cy="18" r="3" />
        <line x1="8.5" y1="7.5" x2="10.5" y2="16.5" />
        <line x1="15.5" y1="7.5" x2="13.5" y2="16.5" />
      </svg>
    ),
  },
];

// ── Category groupings (accordion) ──────────────────────────────

interface SettingsCategory {
  label: string;
  keys: string[];
}

const CATEGORY_I18N_KEYS: Record<string, string> = {
  Business: 'settings-category-business',
  Operations: 'settings-category-operations',
  System: 'settings-category-system',
  Management: 'settings-category-management',
};

const CATEGORIES: SettingsCategory[] = [
  { label: 'Business', keys: ['general', 'appearance'] },
  { label: 'Operations', keys: ['receipt', 'sync', 'email'] },
  { label: 'System', keys: ['about', 'license', 'features', 'data'] },
  { label: 'Management', keys: ['staff', 'terminals', 'stores', 'topology', 'audit', 'offline', 'shifts', 'tax', 'exchange', 'promotions'] },
];

const NAV_L10N_KEYS: Record<string, string> = {
  general: 'settings-nav-general',
  appearance: 'settings-nav-appearance',
  receipt: 'settings-nav-receipt',
  sync: 'settings-nav-sync',
  about: 'settings-nav-about',
  features: 'settings-nav-features',
  data: 'settings-nav-data',
  staff: 'settings-nav-staff',
  terminals: 'settings-nav-terminals',
  stores: 'settings-nav-stores',
  audit: 'settings-nav-audit',
  offline: 'settings-nav-offline',
  shifts: 'settings-nav-shifts',
  tax: 'settings-nav-tax',
  license: 'settings-nav-license',
  exchange: 'settings-nav-exchange',
  promotions: 'settings-nav-promotions',
  email: 'settings-nav-email',
  topology: 'settings-nav-topology',
};

// ── Exported for SettingsPage breadcrumb ────────────────────────

export { NAV_ITEMS, CATEGORIES, CATEGORY_I18N_KEYS, NAV_L10N_KEYS };

// ── Props ────────────────────────────────────────────────────────

interface SettingsNavTreeProps {
  activeSection: string;
  onNavigate: (key: string) => void;
  searchQuery: string;
  onSearchChange: (q: string) => void;
  mobileSidebarOpen: boolean;
  onMobileClose: () => void;
}

// ── Component ─────────────────────────────────────────────────────

export default function SettingsNavTree({
  activeSection,
  onNavigate,
  searchQuery,
  onSearchChange,
  mobileSidebarOpen,
  onMobileClose,
}: SettingsNavTreeProps) {
  const { l10n } = useLocalization();

  // ── Debounced localStorage write (P60-2c: prevents race on rapid toggle) ─
  const debouncedRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  function debouncedPersist(key: string, value: string | null) {
    if (debouncedRef.current) clearTimeout(debouncedRef.current);
    debouncedRef.current = setTimeout(() => {
      if (value === null) {
        localStorage.removeItem(key);
      } else {
        localStorage.setItem(key, value);
      }
      debouncedRef.current = null;
    }, 100);
  }

  useEffect(() => {
    return () => {
      if (debouncedRef.current) clearTimeout(debouncedRef.current);
    };
  }, []);

  // ── Collapsed sidebar (persisted) ──────────────────────────
  const [sidebarCollapsed, setSidebarCollapsed] = useState(() =>
    localStorage.getItem('settings-sidebar-collapsed') === 'true',
  );

  useEffect(() => {
    debouncedPersist('settings-sidebar-collapsed', String(sidebarCollapsed));
  }, [sidebarCollapsed]);

  // ── Accordion: single expanded category (persisted) ──────────
  const [expandedCategory, setExpandedCategory] = useState<string | null>(() => {
    const stored = localStorage.getItem('settings-sidebar-expanded');
    if (stored) return stored;
    return 'Business';
  });

  useEffect(() => {
    debouncedPersist('settings-sidebar-expanded', expandedCategory);
  }, [expandedCategory]);

  // Auto-expand category when navigating to a section
  useEffect(() => {
    const cat = CATEGORIES.find((c) => c.keys.includes(activeSection));
    if (cat?.label) setExpandedCategory(cat.label);
  }, [activeSection]);

  const toggleCategory = useCallback((label: string) => {
    setExpandedCategory((prev) => (prev === label ? null : label));
  }, []);

  // ── Sidebar search filtering ───────────────────────────────
  const q = searchQuery.toLowerCase().trim();
  const filteredCategories = useMemo(() => {
    if (!q) return CATEGORIES;
    return CATEGORIES
      .map((cat) => ({
        ...cat,
        keys: cat.keys.filter((key) => {
          const item = NAV_ITEMS.find((n) => n.key === key);
          return item && (
            item.label.toLowerCase().includes(q) ||
            cat.label.toLowerCase().includes(q)
          );
        }),
      }))
      .filter((cat) => cat.keys.length > 0);
  }, [q]);

  /** Highlight matching characters in a label. */
  const highlightLabel = useCallback((label: string) => {
    if (!q) return label;
    const idx = label.toLowerCase().indexOf(q);
    if (idx === -1) return label;
    return (
      <>
        {label.slice(0, idx)}
        <mark className="settings-nav-highlight">{label.slice(idx, idx + q.length)}</mark>
        {label.slice(idx + q.length)}
      </>
    );
  }, [q]);

  /** Total visible items across all filtered categories. */
  const visibleCount = useMemo(() =>
    filteredCategories.reduce((sum, cat) => sum + cat.keys.length, 0),
  [filteredCategories]);

  // ── Arrow key navigation for sidebar ──────────────────────
  useEffect(() => {
    const flatKeys = filteredCategories.flatMap((c) => c.keys);

    function handleKeyDown(e: KeyboardEvent) {
      // Escape → close mobile sidebar
      if (e.key === 'Escape' && mobileSidebarOpen) {
        e.preventDefault();
        onMobileClose();
        return;
      }

      // Arrow keys navigate sections (skip when focused on inputs)
      const tag = (e.target as HTMLElement)?.tagName;
      if (tag === 'INPUT' || tag === 'SELECT' || tag === 'TEXTAREA') return;

      // P60-2b: Guard against empty search results
      if (flatKeys.length === 0) return;

      if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
        e.preventDefault();
        const idx = flatKeys.indexOf(activeSection);
        if (idx === -1) return;
        const next = e.key === 'ArrowDown'
          ? (idx + 1) % flatKeys.length
          : (idx - 1 + flatKeys.length) % flatKeys.length;
        const nextKey = flatKeys[next];
        if (!nextKey) return;
        onNavigate(nextKey);
      }
    }

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [activeSection, mobileSidebarOpen, filteredCategories, onMobileClose, onNavigate]);

  // ── Render ────────────────────────────────────────────────────

  return (
    <>
      {/* ── Mobile backdrop ─────────────────────── */}
      <div
        className={`settings-sidebar-backdrop${mobileSidebarOpen ? ' visible' : ''}`}
        onClick={onMobileClose}
        aria-hidden="true"
      />

      {/* ── Sidebar ────────────────────────────────── */}
      <aside
        className={`settings-sidebar${sidebarCollapsed ? ' collapsed' : ''}${mobileSidebarOpen ? ' mobile-open' : ''}`}
        data-testid="settings-sidebar"
        aria-label={l10n.getString('settings-sidebar-nav-aria')}
      >
        <div className="settings-sidebar-header">
          <button
            type="button"
            className="settings-sidebar-collapse-all"
            onClick={() => setExpandedCategory(null)}
            aria-label={l10n.getString('settings-sidebar-collapse-all-aria')}
            title={l10n.getString('settings-sidebar-collapse-all-aria')}
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" width="14" height="14">
              <polyline points="6 15 12 9 18 15" />
            </svg>
          </button>
          <button
            type="button"
            className="settings-sidebar-toggle"
            onClick={() => setSidebarCollapsed((p) => !p)}
            aria-label={
              sidebarCollapsed
                ? l10n.getString('settings-sidebar-expand-aria')
                : l10n.getString('settings-sidebar-collapse-aria')
            }
          >
            <svg
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
              width="16"
              height="16"
              aria-hidden="true"
            >
              <polyline points={sidebarCollapsed ? '9 18 15 12 9 6' : '15 18 9 12 15 6'} />
            </svg>
          </button>
        </div>

        <nav className="settings-sidebar-nav">
          {q && filteredCategories.length === 0 ? (
            <div className="settings-sidebar-empty-search">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" width="1.75rem" height="1.75rem" aria-hidden="true">
                <circle cx="11" cy="11" r="8" />
                <line x1="21" y1="21" x2="16.65" y2="16.65" />
                <line x1="8" y1="11" x2="14" y2="11" />
              </svg>
              <Localized id="settings-sidebar-no-results">
                <span className="settings-sidebar-empty-title">No matching sections</span>
              </Localized>
              <button
                type="button"
                className="settings-sidebar-empty-clear"
                onClick={() => onSearchChange('')}
              >
                <Localized id="settings-sidebar-clear-results">Clear search</Localized>
              </button>
            </div>
          ) : (
            filteredCategories.map((cat) => {
              const isExpanded = expandedCategory === cat.label;
              const hasActive = cat.keys.includes(activeSection);
              return (
                <div key={cat.label} className="settings-sidebar-section">
                  <button
                    type="button"
                    className={`settings-sidebar-section-header${hasActive ? ' settings-sidebar-section-header--active' : ''}`}
                    onClick={() => toggleCategory(cat.label)}
                    aria-expanded={isExpanded}
                  >
                    <span className="settings-sidebar-section-label-wrap">
                      <span className="settings-sidebar-section-label">
                        <Localized id={CATEGORY_I18N_KEYS[cat.label] ?? ''}>{cat.label}</Localized>
                      </span>
                      {!sidebarCollapsed && (
                        <span className="settings-sidebar-count" key={cat.keys.length} title={`${cat.keys.length} items`} aria-label={`${cat.keys.length} items`}>
                          {cat.keys.length}
                        </span>
                      )}
                    </span>
                    <svg
                      className={`settings-sidebar-chevron${isExpanded ? '' : ' collapsed'}`}
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      strokeWidth="2"
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      aria-hidden="true"
                    >
                      <polyline points="9 18 15 12 9 6" />
                    </svg>
                  </button>
                  <div className={`settings-sidebar-section-items${isExpanded || sidebarCollapsed ? ' settings-sidebar-section-items--expanded' : ''}`}>
                      {cat.keys.map((key) => {
                        const item = NAV_ITEMS.find((n) => n.key === key)!;
                        return (
                          <Tooltip key={key} content={l10n.getString(NAV_L10N_KEYS[item.key] ?? '')} showDelay={800}>
                            <button
                              type="button"
                              className={`settings-nav-item${activeSection === key ? ' settings-nav-item--active' : ''}`}
                              onClick={() => onNavigate(key)}
                              aria-current={activeSection === key ? 'page' : undefined}
                              aria-label={l10n.getString(NAV_L10N_KEYS[item.key] ?? '')}
                            >
                              <span className="settings-nav-icon">{item.icon}</span>
                              <span className="settings-nav-label">
                                {q ? highlightLabel(l10n.getString(NAV_L10N_KEYS[item.key] ?? item.label)) : (
                                  <Localized id={NAV_L10N_KEYS[item.key] ?? ''}>{item.label}</Localized>
                                )}
                              </span>
                            </button>
                          </Tooltip>
                        );
                      })}
                    </div>
                </div>
              );
            })
          )}
        </nav>
      </aside>

      {/* ── Search results live region (screen readers) ── */}
      <div
        role="status"
        aria-live="polite"
        aria-atomic="true"
        className="sr-only"
      >
        {q && `${visibleCount} ${visibleCount === 1 ? 'result' : 'results'} found`}
      </div>
    </>
  );
}
