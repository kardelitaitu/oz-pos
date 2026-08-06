import { useState, useMemo, useCallback, useEffect, useRef } from 'react';
import { requiredLocalized, LoadingStatus } from '@/frontend/shared';
import { Localized } from '@/components/Localized';
import { type Product } from '@/types/domain';
import { useLocalization } from '@fluent/react';
import { useProducts } from '@/features/products/useProducts';
import { useWorkspaceNav } from '@/hooks/useWorkspaceNav';
import { useAuth } from '@/contexts/AuthContext';
import { useWorkspace } from '@/contexts/WorkspaceContext';
import { useTheme } from '@/frontend/shell/ThemeProvider';
import { useFullscreen } from '@/hooks/useFullscreen';
import { getUserPreferencesScoped, setUserPreferencesScoped } from '@/api/settings';
import './RestaurantMenu.css';

// ── Props ──────────────────────────────────────────────────────────

export interface RestaurantMenuProps {
  /** Called when the user clicks "Add" on a product. */
  onAddProduct?: (product: Product) => void;
}

// ── Helpers ────────────────────────────────────────────────────────

type Category = string;

function key(uid: string, name: string) {
  return `restaurant-${uid}-${name}`;
}

type SortMode = 'manual' | 'a-z' | 'date' | 'popularity';

const COLOR_PALETTE = [
  '#10b981',
  '#ef4444',
  '#f97316',
  '#eab308',
  '#22c55e',
  '#06b6d4',
  '#3b82f6',
  '#8b5cf6',
  '#d946ef',
  '#ec4899',
];

// ── Category icon SVGs ─────────────────────────────────────────────
function CategoryIconFood() {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="14" height="14" aria-hidden="true">
      <path d="M3 2v7c0 1.1.9 2 2 2h4a2 2 0 0 0 2-2V2" />
      <line x1="7" y1="11" x2="7" y2="22" />
      <path d="M21 15V2a5 5 0 0 0-5 5v6c0 1.1.9 2 2 2h3z" />
      <line x1="21" y1="15" x2="21" y2="22" />
    </svg>
  );
}

function CategoryIconSnack() {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="14" height="14" aria-hidden="true">
      <path d="M4 12h16" />
      <path d="M4 12c0 5.5 3.6 9 8 9s8-3.5 8-9" />
      <circle cx="9" cy="9" r="2" fill="currentColor" stroke="none" />
      <circle cx="13" cy="8" r="2" fill="currentColor" stroke="none" />
      <circle cx="17" cy="9" r="2" fill="currentColor" stroke="none" />
    </svg>
  );
}

function CategoryIconHotDrink() {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="14" height="14" aria-hidden="true">
      <path d="M6 8h12l-1.5 12h-9L6 8z" />
      <path d="M17 11h2a2 2 0 0 1 0 4h-2" />
      <path d="M8 8C8.8 6.5 7.2 5.5 8 4" />
      <path d="M13 8C13.8 6.5 12.2 5.5 13 4" />
    </svg>
  );
}

function CategoryIconColdDrink() {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="14" height="14" aria-hidden="true">
      <path d="M5 7h14l-2 15H7L5 7z" />
      <line x1="3" y1="7" x2="21" y2="7" />
      <line x1="16" y1="2" x2="12" y2="22" />
    </svg>
  );
}

function CategoryIconDots1() {
  return (
    <svg viewBox="0 0 16 16" fill="currentColor" width="12" height="12" aria-hidden="true">
      <circle cx="8" cy="8" r="3" />
    </svg>
  );
}
function CategoryIconDots2() {
  return (
    <svg viewBox="0 0 16 16" fill="currentColor" width="12" height="12" aria-hidden="true">
      <circle cx="5" cy="8" r="2.5" />
      <circle cx="11" cy="8" r="2.5" />
    </svg>
  );
}
function CategoryIconDots3() {
  return (
    <svg viewBox="0 0 16 16" fill="currentColor" width="12" height="12" aria-hidden="true">
      <circle cx="3" cy="8" r="2" />
      <circle cx="8" cy="8" r="2" />
      <circle cx="13" cy="8" r="2" />
    </svg>
  );
}

function CategoryIcon({ icon }: { icon: string }) {
  if (icon === 'food')       return <CategoryIconFood />;
  if (icon === 'snack')      return <CategoryIconSnack />;
  if (icon === 'hot-drink')  return <CategoryIconHotDrink />;
  if (icon === 'cold-drink') return <CategoryIconColdDrink />;
  if (icon === 'dots-1')     return <CategoryIconDots1 />;
  if (icon === 'dots-2')     return <CategoryIconDots2 />;
  if (icon === 'dots-3')     return <CategoryIconDots3 />;
  return null;
}

function loadPinned(uid: string): Set<string> {
  try {
    const raw = localStorage.getItem(key(uid, 'pinned'));
    return new Set<string>(raw ? JSON.parse(raw) : []);
  } catch {
    return new Set();
  }
}

function savePinned(pinned: Set<string>, uid: string) {
  localStorage.setItem(key(uid, 'pinned'), JSON.stringify([...pinned]));
}

function loadColors(uid: string): Record<string, string> {
  try {
    const raw = localStorage.getItem(key(uid, 'colors'));
    return raw ? JSON.parse(raw) : {};
  } catch {
    return {};
  }
}

function saveColors(colors: Record<string, string>, uid: string) {
  localStorage.setItem(key(uid, 'colors'), JSON.stringify(colors));
}

function loadPop(uid: string): Record<string, number> {
  try {
    const raw = localStorage.getItem(key(uid, 'pop'));
    return raw ? JSON.parse(raw) : {};
  } catch {
    return {};
  }
}

function savePop(pop: Record<string, number>, uid: string) {
  try { localStorage.setItem(key(uid, 'pop'), JSON.stringify(pop)); } catch { /* quota */ }
}

function loadUnavailable(uid: string): Set<string> {
  try {
    const raw = localStorage.getItem(key(uid, 'unavail'));
    return new Set<string>(raw ? JSON.parse(raw) : []);
  } catch {
    return new Set();
  }
}
function saveUnavailable(unavail: Set<string>, uid: string) {
  localStorage.setItem(key(uid, 'unavail'), JSON.stringify([...unavail]));
}

/** Sort so pinned items appear first, preserving original order within each group. */
function sortPinnedFirst(items: Product[], pinned: Set<string>): Product[] {
  const pinnedList: Product[] = [];
  const rest: Product[] = [];
  for (const p of items) {
    if (pinned.has(p.sku)) pinnedList.push(p);
    else rest.push(p);
  }
  return [...pinnedList, ...rest];
}

/** Plus icon SVG */
function PlusIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <line x1="12" y1="5" x2="12" y2="19" />
      <line x1="5" y1="12" x2="19" y2="12" />
    </svg>
  );
}

// ── Pin icon SVG ──────────────────────────────────────────────────

function PinIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="currentColor" width="12" height="12" aria-hidden="true">
      <path d="M16 12V4h1V2H7v2h1v8l-2 2v2h5.2v6h1.6v-6H18v-2l-2-2z" />
    </svg>
  );
}

// ── Component ──────────────────────────────────────────────────────

/**
 * Restaurant-style menu panel for the POS.
 *
 * Shows a search box, horizontal scrollable row of category pills,
 * and a responsive product grid. Right-click any item to pin it to
 * the top of the grid.
 */
export default function RestaurantMenu({ onAddProduct }: RestaurantMenuProps) {
  const { l10n } = useLocalization();
  const { products, categoryMeta, loading } = useProducts();
  const { goToWorkspacePicker } = useWorkspaceNav();
  const { session, logout } = useAuth();
  const { sessionToken } = useWorkspace();
  const userId = session?.user_id ?? 'default';
  const { theme, toggleTheme } = useTheme();
  const [menuOpen, setMenuOpen] = useState(false);
  const hamburgerRef = useRef<HTMLDivElement>(null);

  // Close hamburger menu on click outside
  useEffect(() => {
    if (!menuOpen) return;
    const handler = (e: MouseEvent) => {
      if (hamburgerRef.current && !hamburgerRef.current.contains(e.target as Node)) {
        setMenuOpen(false);
      }
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, [menuOpen]);

  const [addedSku, setAddedSku] = useState<string | null>(null);
  const addedTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const handleAddProduct = useCallback((product: Product) => {
    addCountRef.current = { ...addCountRef.current, [product.sku]: (addCountRef.current[product.sku] ?? 0) + 1 };
    savePop(addCountRef.current, userId);
    forceRender((n) => n + 1);
    onAddProduct?.(product);
    setAddedSku(product.sku);
    if (addedTimerRef.current) clearTimeout(addedTimerRef.current);
    addedTimerRef.current = setTimeout(() => setAddedSku(null), 400);
  }, [onAddProduct, userId]);

  const { toggleFullscreen } = useFullscreen();
  const [activeCategory, setActiveCategory] = useState<Category>('All');
  const [searchQuery, setSearchQuery] = useState('');
  const searchInputRef = useRef<HTMLInputElement>(null);
  const [pinned, setPinned] = useState<Set<string>>(loadPinned(userId));
  const [colors, setColors] = useState<Record<string, string>>(loadColors(userId));
  const [unavailable, setUnavailable] = useState<Set<string>>(loadUnavailable(userId));
  const addCountRef = useRef<Record<string, number>>(loadPop(userId));
  const [, forceRender] = useState(0);
  const [sortMode, setSortMode] = useState<SortMode>(() => {
    try {
      const stored = localStorage.getItem(key(userId, 'sort'));
      return stored === 'a-z' || stored === 'date' || stored === 'popularity' ? stored : 'manual';
    } catch { return 'manual'; }
  });
  const [cardSize, setCardSize] = useState(() => {
    try { return Math.min(4, Math.max(0, parseInt(localStorage.getItem(key(userId, 'cardsize')) ?? '0', 10) || 0)); }
    catch { return 0; }
  });
  const [fontSize, setFontSize] = useState(() => {
    try { return Math.min(4, Math.max(0, parseInt(localStorage.getItem(key(userId, 'fontsize')) ?? '0', 10) || 0)); }
    catch { return 0; }
  });

  // Clean up add-to-cart animation timer on unmount
  useEffect(() => {
    return () => {
      if (addedTimerRef.current) clearTimeout(addedTimerRef.current);
    };
  }, []);

  const persistMenuPreference = useCallback((preferenceKey: string, value: string) => {
    try { localStorage.setItem(key(userId, preferenceKey), value); } catch { /* offline / quota */ }
    if (!sessionToken) return;
    void setUserPreferencesScoped(sessionToken, [{ key: preferenceKey, value }]).catch(() => {
      // Keep the local preference when the scoped backend is temporarily offline.
    });
  }, [sessionToken, userId]);

  const changeCardSize = useCallback((delta: number) => {
    const next = Math.min(4, Math.max(0, cardSize + delta));
    if (next === cardSize) return;
    setCardSize(next);
    persistMenuPreference('cardsize', String(next));
  }, [cardSize, persistMenuPreference]);

  const changeFontSize = useCallback((delta: number) => {
    const next = Math.min(4, Math.max(0, fontSize + delta));
    if (next === fontSize) return;
    setFontSize(next);
    persistMenuPreference('fontsize', String(next));
  }, [fontSize, persistMenuPreference]);

  // Load preferences from backend on mount, syncing to localStorage. Ignore
  // late responses when the active user or store session changes.
  useEffect(() => {
    if (!sessionToken) return;
    let cancelled = false;
    getUserPreferencesScoped(sessionToken).then((prefs) => {
      if (cancelled) return;
      const cs = prefs['cardsize'];
      if (cs !== undefined) {
        const v = Math.min(4, Math.max(0, parseInt(cs, 10) || 0));
        setCardSize(v);
        try { localStorage.setItem(key(userId, 'cardsize'), String(v)); } catch { /* noop */ }
      }
      const fs = prefs['fontsize'];
      if (fs !== undefined) {
        const v = Math.min(4, Math.max(0, parseInt(fs, 10) || 0));
        setFontSize(v);
        try { localStorage.setItem(key(userId, 'fontsize'), String(v)); } catch { /* noop */ }
      }
      const savedSort = prefs['sort'];
      if (savedSort === 'manual' || savedSort === 'a-z' || savedSort === 'date' || savedSort === 'popularity') {
        setSortMode(savedSort);
        try { localStorage.setItem(key(userId, 'sort'), savedSort); } catch { /* noop */ }
      }
      const fsm = prefs['font-smoothing'];
      if (fsm === 'antialiased' || fsm === 'subpixel') {
        document.documentElement.setAttribute('data-font-smoothing', fsm);
      }
    }).catch(() => { /* offline — keep localStorage values */ });
    return () => { cancelled = true; };
  }, [userId, sessionToken]);

  // Sync pinned/colors/unavailable to localStorage whenever state changes.
  // Extracted from setState updaters to avoid side effects in pure functions
  // (React 18 strict mode calls updaters twice, causing duplicate writes).
  useEffect(() => { savePinned(pinned, userId); }, [pinned, userId]);
  useEffect(() => { saveColors(colors, userId); }, [colors, userId]);
  useEffect(() => { saveUnavailable(unavailable, userId); }, [unavailable, userId]);

  // ── Context menu state ──────────────────────────────
  const [contextMenu, setContextMenu] = useState<{
    sku: string;
    x: number;
    y: number;
    isPinned: boolean;
    isUnavailable: boolean;
    currentColor: string | undefined;
    fromKeyboard: boolean;
  } | null>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  // A11Y-06: the card that opened the menu (for focus restoration) and whether
  // it was opened via keyboard (Shift+F10 / ContextMenu key) — only the
  // keyboard path should move focus into the menu and restore it on close.
  const contextTriggerRef = useRef<HTMLElement | null>(null);
  const contextFromKeyboardRef = useRef(false);

  // A11Y-06: close the menu and, when it was opened from the keyboard, return
  // focus to the triggering card (WAI-ARIA menu pattern).
  const closeContextMenu = useCallback(() => {
    setContextMenu(null);
    if (contextFromKeyboardRef.current) contextTriggerRef.current?.focus();
    contextTriggerRef.current = null;
    contextFromKeyboardRef.current = false;
  }, []);

  // A11Y-06: keyboard-opened menus move focus to the first menuitem so the
  // menu is immediately operable without an extra Tab.
  useEffect(() => {
    if (!contextMenu?.fromKeyboard) return;
    menuRef.current?.querySelector<HTMLElement>('button[role="menuitem"]')?.focus();
  }, [contextMenu]);

  // A11Y-06: ArrowUp / ArrowDown roving focus between menuitems (wraps).
  // When no menuitem currently has focus (idx === -1), ArrowDown lands on the
  // first item and ArrowUp wraps to the last — robust for any item count.
  const handleContextMenuKeyDown = useCallback((e: React.KeyboardEvent<HTMLDivElement>) => {
    if (e.key !== 'ArrowDown' && e.key !== 'ArrowUp') return;
    const items = Array.from(
      e.currentTarget.querySelectorAll<HTMLElement>('button[role="menuitem"]'),
    );
    if (items.length === 0) return;
    const idx = items.indexOf(document.activeElement as HTMLElement);
    e.preventDefault();
    if (e.key === 'ArrowDown') {
      items[idx === -1 ? 0 : (idx + 1) % items.length]?.focus();
    } else {
      items[idx === -1 ? items.length - 1 : (idx - 1 + items.length) % items.length]?.focus();
    }
  }, []);

  // Close context menu on click outside or Escape (Escape restores focus).
  useEffect(() => {
    if (!contextMenu) return;
    const handler = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        closeContextMenu();
      }
    };
    const keyHandler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') closeContextMenu();
    };
    document.addEventListener('mousedown', handler);
    document.addEventListener('keydown', keyHandler);
    return () => {
      document.removeEventListener('mousedown', handler);
      document.removeEventListener('keydown', keyHandler);
    };
  }, [contextMenu, closeContextMenu]);

  // Listen for global Ctrl+F → focus search input
  useEffect(() => {
    const handler = () => searchInputRef.current?.focus();
    window.addEventListener('app-search', handler);
    return () => window.removeEventListener('app-search', handler);
  }, []);

  // Type anywhere → focus search + insert char, Escape → clear + blur
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        setSearchQuery('');
        searchInputRef.current?.blur();
        return;
      }
      const tag = (e.target as HTMLElement)?.tagName;
      if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return;
      if (e.ctrlKey || e.metaKey || e.altKey || e.key.length !== 1) return;

      e.preventDefault();
      const input = searchInputRef.current;
      if (!input) return;
      input.focus();
      const selStart = input.selectionStart ?? input.value.length;
      const selEnd = input.selectionEnd ?? selStart;
      const newVal = input.value.slice(0, selStart) + e.key + input.value.slice(selEnd);
      const nativeSetter = Object.getOwnPropertyDescriptor(
        window.HTMLInputElement.prototype, 'value'
      )?.set;
      nativeSetter?.call(input, newVal);
      input.dispatchEvent(new Event('input', { bubbles: true }));
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, []);

  const handleContextMenu = useCallback((sku: string, e: React.MouseEvent, trigger?: HTMLElement, fromKeyboard = false) => {
    e.preventDefault();
    contextTriggerRef.current = trigger ?? null;
    contextFromKeyboardRef.current = fromKeyboard;
    setContextMenu({ sku, x: e.clientX, y: e.clientY, isPinned: pinned.has(sku), isUnavailable: unavailable.has(sku), currentColor: colors[sku], fromKeyboard });
  }, [pinned, colors, unavailable]);

  const togglePin = useCallback((sku: string) => {
    setPinned((prev) => {
      const next = new Set(prev);
      if (next.has(sku)) next.delete(sku);
      else next.add(sku);
      return next;
    });
    closeContextMenu();
  }, [closeContextMenu]);

  const toggleUnavailable = useCallback((sku: string) => {
    setUnavailable((prev) => {
      const next = new Set(prev);
      if (next.has(sku)) next.delete(sku);
      else next.add(sku);
      return next;
    });
    closeContextMenu();
  }, [closeContextMenu]);

  const setColor = useCallback((sku: string, color: string) => {
    setColors((prev) => {
      if (color === prev[sku]) return prev;
      return { ...prev, [sku]: color };
    });
    closeContextMenu();
  }, [closeContextMenu]);

  const clearColor = useCallback((sku: string) => {
    setColors((prev) => {
      if (!(sku in prev)) return prev;
      const next = { ...prev };
      delete next[sku];
      return next;
    });
    closeContextMenu();
  }, [closeContextMenu]);

  // Restaurant and retail are separate workspaces. Category tabs must be
  // derived from restaurant products rather than the shared catalog category
  // list, otherwise a retail-only category can leak into this menu.
  const restaurantProducts = useMemo(
    () => products.filter((p) => p.productType === 'restaurant'),
    [products],
  );

  const categoryOptions = useMemo<Category[]>(
    () => ['All', ...new Set(restaurantProducts.map((p) => p.category))].sort((a, b) => {
      if (a === 'All') return -1;
      if (b === 'All') return 1;
      return a.localeCompare(b);
    }),
    [restaurantProducts],
  );

  // Keep the selection valid when products are refreshed or removed. The
  // derived value prevents a transient empty result while the state catches
  // up; the effect also clears the stale selection if that category returns.
  const effectiveCategory = categoryOptions.includes(activeCategory) ? activeCategory : 'All';
  useEffect(() => {
    if (activeCategory !== 'All' && !categoryOptions.includes(activeCategory)) {
      setActiveCategory('All');
    }
  }, [activeCategory, categoryOptions]);

  const catMetaMap = useMemo(() => {
    const m = new Map<string, { colour: string; icon: string }>();
    for (const c of categoryMeta) m.set(c.name, { colour: c.colour, icon: c.icon });
    return m;
  }, [categoryMeta]);

  const filtered = useMemo(() => {
    let result = restaurantProducts;
    if (effectiveCategory !== 'All') result = result.filter((p) => p.category === effectiveCategory);
    if (searchQuery.trim()) {
      const q = searchQuery.trim().toLowerCase();
      result = result.filter((p) => p.name.toLowerCase().includes(q) || p.sku.toLowerCase().includes(q));
    }
    result = [...result];
    const counts = sortMode === 'popularity' ? addCountRef.current : undefined;
    switch (sortMode) {
      case 'a-z':
        result.sort((a, b) => a.name.localeCompare(b.name, undefined, { sensitivity: 'base' }));
        break;
      case 'date':
        result.sort((a, b) => (b.createdAt ?? '').localeCompare(a.createdAt ?? ''));
        break;
      case 'popularity':
        result.sort((a, b) => (counts?.[b.sku] ?? 0) - (counts?.[a.sku] ?? 0));
        break;
    }
    return sortPinnedFirst(result, pinned);
  }, [effectiveCategory, restaurantProducts, searchQuery, pinned, sortMode]);

  return (
    <div className="restaurant-menu" style={{ '--card-size': cardSize, '--font-size': fontSize } as React.CSSProperties}>
      {/* ── Header row: hamburger + back + search ── */}
      <div className="restaurant-header">
          <div className="restaurant-header-left" ref={hamburgerRef}>
          <button
            type="button"
            className="restaurant-hamburger-btn"
            onClick={() => setMenuOpen((prev) => !prev)}
            aria-label={l10n.getString('restaurant-menu-hamburger-aria')}
            aria-expanded={menuOpen}
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="20" height="20" style={{ pointerEvents: 'none' }}>
              <line x1="3" y1="6" x2="21" y2="6" />
              <line x1="3" y1="12" x2="21" y2="12" />
              <line x1="3" y1="18" x2="21" y2="18" />
            </svg>
          </button>

          {menuOpen && (
            <div className="restaurant-hamburger-dropdown" role="menu">
              <span className="restaurant-hamburger-label"><Localized id="restaurant-sort-label"><span>Sort</span></Localized></span>
              {(['manual', 'a-z', 'date', 'popularity'] as const).map((mode) => (
                <button
                  key={mode}
                  type="button"
                  className="restaurant-hamburger-item restaurant-hamburger-item--sort"
                  role="menuitem"
                  onClick={() => {
                    setSortMode(mode);
                    persistMenuPreference('sort', mode);
                    setMenuOpen(false);
                  }}
                >
                  {sortMode === mode && <span className="restaurant-sort-check">✓</span>}
                  <Localized id={`restaurant-sort-${mode}`}>
                    <span>{mode === 'manual' ? 'Manual' : mode === 'a-z' ? 'A–Z' : mode === 'date' ? 'By Date' : 'Popularity'}</span>
                  </Localized>
                </button>
              ))}
              <div className="restaurant-hamburger-divider" role="separator" />
              <div className="restaurant-hamburger-item restaurant-hamburger-size" role="menuitem">
                <span className="restaurant-hamburger-size-label"><Localized id="restaurant-size-label"><span>Menu Size</span></Localized></span>
                <div className="restaurant-hamburger-size-controls">
                  <button
                    type="button"
                    className="restaurant-size-btn"
                    disabled={cardSize <= 0}
                    onClick={() => changeCardSize(-1)}
                    aria-label={l10n.getString('restaurant-size-decrease-aria')}
                  >
                    &minus;
                  </button>
                  <span className="restaurant-size-value">{cardSize}</span>
                  <button
                    type="button"
                    className="restaurant-size-btn"
                    disabled={cardSize >= 4}
                    onClick={() => changeCardSize(1)}
                    aria-label={l10n.getString('restaurant-size-increase-aria')}
                  >
                    +
                  </button>
                </div>
              </div>
              <div className="restaurant-hamburger-divider" role="separator" />
              <div className="restaurant-hamburger-item restaurant-hamburger-size" role="menuitem">
                <Localized id="restaurant-font-size-label">
                  <span className="restaurant-hamburger-size-label">Font Size</span>
                </Localized>
                <div className="restaurant-hamburger-size-controls">
                  <button
                    type="button"
                    className="restaurant-size-btn"
                    disabled={fontSize <= 0}
                    onClick={() => changeFontSize(-1)}
                    aria-label={l10n.getString('restaurant-font-size-decrease-aria')}
                  >
                    &minus;
                  </button>
                  <span className="restaurant-size-value">{fontSize}</span>
                  <button
                    type="button"
                    className="restaurant-size-btn"
                    disabled={fontSize >= 4}
                    onClick={() => changeFontSize(1)}
                    aria-label={l10n.getString('restaurant-font-size-increase-aria')}
                  >
                    +
                  </button>
                </div>
              </div>
              <div className="restaurant-hamburger-divider" role="separator" />
              <button
                type="button"
                className="restaurant-hamburger-item"
                role="menuitem"
                aria-label={l10n.getString(theme === 'dark' ? 'restaurant-theme-light' : 'restaurant-theme-dark')}
                onClick={() => { toggleTheme(); setMenuOpen(false); }}
              >
                <Localized id={theme === 'dark' ? 'restaurant-theme-light' : 'restaurant-theme-dark'}>
                  <span>{theme === 'dark' ? 'Light Mode' : 'Dark Mode'}</span>
                </Localized>
              </button>
              <button
                type="button"
                className="restaurant-hamburger-item"
                role="menuitem"
                aria-label={l10n.getString('restaurant-lock-terminal')}
                onClick={() => { logout(); setMenuOpen(false); }}
              >
                <Localized id="restaurant-lock-terminal"><span>Lock Terminal</span></Localized>
              </button>
              <button
                type="button"
                className="restaurant-hamburger-item"
                role="menuitem"
                aria-label={l10n.getString('restaurant-toggle-fullscreen')}
                onClick={() => { toggleFullscreen(); setMenuOpen(false); }}
              >
                <Localized id="restaurant-toggle-fullscreen"><span>Toggle Fullscreen</span></Localized>
              </button>
            </div>
          )}
        </div>

        <button
          type="button"
          className="restaurant-back-btn"
          onClick={goToWorkspacePicker}
          aria-label={l10n.getString('restaurant-menu-back-aria')}
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="18" height="18" style={{ pointerEvents: 'none' }}>
            <polyline points="15 18 9 12 15 6" />
          </svg>
        </button>
        <div className="restaurant-search">
        <svg className="restaurant-search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" width="16" height="16" aria-hidden="true">
          <circle cx="11" cy="11" r="8" />
          <line x1="21" y1="21" x2="16.65" y2="16.65" />
        </svg>
        <input
          type="search"
          className="restaurant-search-input"
          id="restaurant-menu-search"
          name="restaurant-menu-search"
          autoComplete="off"
          autoCorrect="off"
          spellCheck={false}
          data-1p-ignore="true"
          data-lpignore="true"
          data-bwignore="true"
          ref={searchInputRef}
          placeholder={l10n.getString('restaurant-menu-search-placeholder')}
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          aria-label={l10n.getString('restaurant-search-aria')}
        />
        {searchQuery && (
          <button
            type="button"
            className="restaurant-search-clear"
            onClick={() => setSearchQuery('')}
            aria-label={l10n.getString('restaurant-search-clear-aria')}
          >
            &times;
          </button>
        )}
      </div>
      </div>

      {/* ── Category pills ─────────────────────────── */}
      <div className="restaurant-categories" role="tablist" aria-label={l10n.getString('restaurant-categories-aria')}>
          {categoryOptions.map((cat) => {
            const meta = catMetaMap.get(cat);
            const isActive = effectiveCategory === cat;
            return (
              <button
                key={cat}
                type="button"
                role="tab"
                aria-selected={isActive}
                className={
                  isActive
                    ? 'restaurant-category-pill restaurant-category-pill--active'
                    : 'restaurant-category-pill'
                }
                style={meta && isActive
                  ? { '--pill-color': meta.colour } as React.CSSProperties
                  : undefined}
                onClick={() => setActiveCategory(cat)}
              >
                {meta?.icon && (
                  <span className="restaurant-pill-icon">
                    <CategoryIcon icon={meta.icon} />
                  </span>
                )}
                {cat}
              </button>
            );
          })}        </div>

      {/* ── Product grid ───────────────────────────── */}
      {loading ? (
        // LOAD-05: localized status announcement for the loading region.
        <LoadingStatus
          className="restaurant-empty"
          label={requiredLocalized(l10n, 'restaurant-menu-loading')}
        />
      ) : filtered.length === 0 ? (
        <div className="restaurant-empty">
          <span className="restaurant-empty-text">
            <Localized id="restaurant-menu-empty">
              <span>No items available</span>
            </Localized>
          </span>
        </div>
      ) : (
        <div className="restaurant-grid" role="list" aria-label={requiredLocalized(l10n, 'restaurant-menu-items-aria')}>
          {filtered.map((product, i) => (
            <RestaurantCard
              key={product.sku}
              product={{ ...product, inStock: product.inStock && !unavailable.has(product.sku) }}
              pinned={pinned.has(product.sku)}
              color={colors[product.sku] ?? catMetaMap.get(product.category)?.colour}
              onAdd={handleAddProduct}
              onContextMenu={handleContextMenu}
              added={product.sku === addedSku}
              index={i}
            />
          ))}
        </div>
      )}

      {/* ── Context menu ─────────────────────────────── */}
      {contextMenu && (
        <div
          ref={menuRef}
          className="restaurant-context-menu"
          style={{
            left: Math.max(4, Math.min(contextMenu.x, window.innerWidth - 180)),
            top: Math.max(4, Math.min(contextMenu.y, window.innerHeight - 280)),
          }}
          role="menu"
          tabIndex={-1}
          onKeyDown={handleContextMenuKeyDown}
        >
          <button
            type="button"
            className="restaurant-context-item"
            aria-label={l10n.getString(contextMenu.isPinned ? 'restaurant-context-unpin' : 'restaurant-context-pin')}
            onClick={() => togglePin(contextMenu.sku)}
            role="menuitem"
          >
            <Localized id={contextMenu.isPinned ? 'restaurant-context-unpin' : 'restaurant-context-pin'}>
            <span>{contextMenu.isPinned ? 'Unpin from top' : 'Pin to top'}</span>
          </Localized>
          </button>
          <button
            type="button"
            className="restaurant-context-item"
            aria-label={l10n.getString(contextMenu.isUnavailable ? 'restaurant-context-available' : 'restaurant-context-unavailable')}
            onClick={() => toggleUnavailable(contextMenu.sku)}
            role="menuitem"
          >
            <Localized id={contextMenu.isUnavailable ? 'restaurant-context-available' : 'restaurant-context-unavailable'}>
            <span>{contextMenu.isUnavailable ? 'Mark available' : 'Mark unavailable'}</span>
          </Localized>
          </button>
          <div className="restaurant-context-divider" role="separator" />
          <span className="restaurant-context-label"><Localized id="restaurant-context-color-label"><span>Colorize Add</span></Localized></span>
          <div className="restaurant-context-colors" role="menuitem">
            {COLOR_PALETTE.map((c) => (
              <button
                key={c}
                type="button"
                className={`restaurant-context-swatch${contextMenu.currentColor === c ? ' restaurant-context-swatch--active' : ''}`}
                style={{ background: c }}
                onClick={() => setColor(contextMenu.sku, c)}
                aria-label={l10n.getString('restaurant-color-swatch-aria', { color: c }, c)}
              />
            ))}
            {contextMenu.currentColor && (
              <button
                type="button"
                className="restaurant-context-swatch restaurant-context-swatch--clear"
                onClick={() => clearColor(contextMenu.sku)}
                aria-label={l10n.getString('restaurant-clear-color-aria')}
              >
                ✕
              </button>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

// ── RestaurantCard sub-component ───────────────────────────────────

interface RestaurantCardProps {
  product: Product;
  pinned: boolean;
  color: string | undefined;
  onAdd: ((product: Product) => void) | undefined;
  onContextMenu: (sku: string, e: React.MouseEvent, trigger: HTMLElement, fromKeyboard: boolean) => void;
  added?: boolean;
  index?: number;
}

function RestaurantCard({ product, pinned, color, onAdd, onContextMenu, added, index }: RestaurantCardProps) {
  const { l10n } = useLocalization();
  const handleClick = useCallback(() => {
    // Unavailable cards remain context-menu accessible so they can be marked
    // available again, but must never add an out-of-stock item to the sale.
    if (!product.inStock) return;
    onAdd?.(product);
  }, [product, onAdd]);

  const handleContext = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    onContextMenu(product.sku, e, e.currentTarget as HTMLElement, false);
  }, [product.sku, onContextMenu]);

  const handleKeyDown = useCallback((e: React.KeyboardEvent<HTMLButtonElement>) => {
    if (e.key === 'ContextMenu' || (e.shiftKey && e.key === 'F10')) {
      e.preventDefault();
      const rect = e.currentTarget.getBoundingClientRect();
      onContextMenu(
        product.sku,
        {
          preventDefault: () => {},
          clientX: rect.right,
          clientY: rect.top,
        } as unknown as React.MouseEvent,
        e.currentTarget,
        true,
      );
    }
  }, [product.sku, onContextMenu]);

  let cardClass = 'restaurant-card';
  if (pinned) cardClass += ' restaurant-card--pinned';
  if (!product.inStock) cardClass += ' restaurant-card--disabled';
  if (added) cardClass += ' restaurant-card--added';

  return (
    <button
      type="button"
      className={cardClass}
      tabIndex={0}
      aria-disabled={!product.inStock}
      onClick={handleClick}
      onContextMenu={handleContext}
      onKeyDown={handleKeyDown}
      style={{ '--btn-color': color ?? 'var(--color-accent)', animationDelay: `${(index ?? 0) * 35}ms` } as React.CSSProperties}
    >
      <div className="restaurant-card-body">
        {pinned && (
          <span className="restaurant-card-pin-badge" title={requiredLocalized(l10n, 'restaurant-card-pin-title')}>
            <PinIcon />
          </span>
        )}
        <h3 className="restaurant-card-name" title={product.name}>
          {product.name}
        </h3>
      </div>
      <span className="restaurant-card-add-icon" aria-hidden="true">
        <PlusIcon />
        <Localized id="restaurant-card-add">
          <span>Add</span>
        </Localized>
      </span>
    </button>
  );
}
