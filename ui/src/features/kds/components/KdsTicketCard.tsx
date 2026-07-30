import { useEffect, useRef, useState, memo, useCallback } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import { useTicketSla } from '@/features/kds/hooks/useTicketSla';
import { useSound } from '@/frontend/shared/useSound';
import { getKdsOrderLinesScoped, type KdsOrder, type KdsStatus, type KdsLineItem } from '@/api/kds';

/** Props for the KdsTicketCard component. */
export interface KdsTicketCardProps {
  /** The KDS order data to display. */
  order: KdsOrder;
  /** Called when the ticket is tapped to advance to the next status. */
  onAdvance: (order: KdsOrder) => void;
  /** Whether to show the order number (#123). */
  showOrderId?: boolean;
  /** Whether to show the table number. */
  showTableNumber?: boolean;
  /** Whether this ticket is keyboard-selected (highlighted). */
  selected?: boolean;
  /** Called when the items on this ticket are edited. */
  onSaveItems?: (orderId: string, itemsSummary: string, itemCount: number) => void;
  /** Session token for scoped API calls (e.g., fetching line items). */
  sessionToken: string;
}

/** Course display order — items without a course map to "other" at the end. */
const COURSE_ORDER = ['appetizer', 'main', 'side', 'dessert', 'beverage'] as const;

const COURSE_L10N_KEYS: Record<string, string> = {
  appetizer: 'kds-course-appetizer',
  main: 'kds-course-main',
  side: 'kds-course-side',
  dessert: 'kds-course-dessert',
  beverage: 'kds-course-beverage',
};

/** Group line items by course, preserving course order. Returns entries in display order. */
function groupByCourse(items: KdsLineItem[]): { course: string | null; items: KdsLineItem[] }[] {
  const groups = new Map<string | null, KdsLineItem[]>();
  for (const item of items) {
    const course = item.course ?? null;
    if (!groups.has(course)) groups.set(course, []);
    groups.get(course)!.push(item);
  }

  const ordered: { course: string | null; items: KdsLineItem[] }[] = [];
  for (const c of COURSE_ORDER) {
    if (groups.has(c)) {
      ordered.push({ course: c, items: groups.get(c)! });
      groups.delete(c);
    }
  }
  // Remaining courses (including null/unknown) come last.
  for (const [course, courseItems] of groups) {
    ordered.push({ course, items: courseItems });
  }
  return ordered;
}

const STATUS_ORDER: KdsStatus[] = ['pending', 'preparing', 'ready', 'served'];

/**
 * KdsTicketCard renders a single KDS ticket with SLA aging indicators,
 * course-grouped line items (Phase 2), and audio alerts at escalation thresholds.
 *
 * Line items are lazy-fetched via getKdsOrderLinesScoped when the card mounts
 * and cached for the lifetime of the component. Falls back to the flat
 * items_summary string when line items are unavailable (old orders, loading).
 */
export const KdsTicketCard = memo(function KdsTicketCard({
  order, onAdvance, showOrderId = true, showTableNumber = true,
  selected = false, onSaveItems, sessionToken,
}: KdsTicketCardProps) {
  const { l10n } = useLocalization();
  const { level, urgent, display } = useTicketSla(order.received_at);
  const { playAlert } = useSound();
  const prevLevel = useRef<'green' | 'yellow' | 'red' | null>(null);
  const prevUrgent = useRef(false);

  // Play audio alert when ticket transitions into the red threshold.
  useEffect(() => {
    if (prevLevel.current !== null && prevLevel.current !== 'red' && level === 'red') {
      playAlert();
    }
    prevLevel.current = level;
  }, [level, playAlert]);

  // Play a second alert when ticket escalates to red-urgent (≥15 min).
  useEffect(() => {
    if (urgent && !prevUrgent.current) {
      playAlert();
    }
    prevUrgent.current = urgent;
  }, [urgent, playAlert]);

  const [editing, setEditing] = useState(false);
  const [editSummary, setEditSummary] = useState(order.items_summary);
  const [editCount, setEditCount] = useState(String(order.item_count));
  const inputRef = useRef<HTMLInputElement>(null);

  // ── Line items: lazy-fetch + cache ──────────────────────────────
  const [lineItems, setLineItems] = useState<KdsLineItem[] | null>(null);
  const [lineItemsLoading, setLineItemsLoading] = useState(false);
  const lineItemsFetched = useRef(false);

  useEffect(() => {
    // Only fetch once per mount. If line items are already loaded, don't re-fetch.
    if (lineItemsFetched.current) return;
    lineItemsFetched.current = true;
    setLineItemsLoading(true);

    getKdsOrderLinesScoped(sessionToken, order.id)
      .then((items) => {
        setLineItems(items);
        setLineItemsLoading(false);
      })
      .catch(() => {
        // Silently fall back to items_summary — the flat display works for all orders.
        setLineItemsLoading(false);
      });
  }, [sessionToken, order.id]);

  // Group items by course for structured display.
  const courseGroups = lineItems && lineItems.length > 0
    ? groupByCourse(lineItems)
    : null;

  // ── Edit state ───────────────────────────────────────────────────

  // Sync edit state when order changes (e.g., after save).
  useEffect(() => {
    if (!editing) {
      setEditSummary(order.items_summary);
      setEditCount(String(order.item_count));
    }
  }, [order.items_summary, order.item_count, editing]);

  // Focus the text input when edit mode opens.
  useEffect(() => {
    if (editing) inputRef.current?.focus();
  }, [editing]);

  const handleSaveEdit = useCallback(() => {
    const parsed = parseInt(editCount, 10);
    if (!editSummary.trim() || isNaN(parsed) || parsed <= 0) return;
    onSaveItems?.(order.id, editSummary.trim(), parsed);
    setEditing(false);
  }, [editSummary, editCount, onSaveItems, order.id]);

  const handleCancelEdit = useCallback(() => {
    setEditSummary(order.items_summary);
    setEditCount(String(order.item_count));
    setEditing(false);
  }, [order.items_summary, order.item_count]);

  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (e.key === 'Enter') handleSaveEdit();
    if (e.key === 'Escape') handleCancelEdit();
  }, [handleSaveEdit, handleCancelEdit]);

  const handleClick = () => {
    if (editing) return;
    const currentIdx = STATUS_ORDER.indexOf(order.status as KdsStatus);
    if (currentIdx >= 0 && currentIdx < STATUS_ORDER.length - 1) {
      onAdvance(order);
    }
  };

  const startEditing = (e: React.MouseEvent) => {
    e.stopPropagation();
    setEditing(true);
  };

  // ── Course label resolver ────────────────────────────────────────
  const courseLabel = useCallback((course: string | null): string => {
    if (!course) return l10n.getString('kds-course-other') || 'OTHER';
    const key = COURSE_L10N_KEYS[course];
    if (key) return l10n.getString(key) || course.toUpperCase();
    return l10n.getString('kds-course-other') || 'OTHER';
  }, [l10n]);

  return (
    <button
      className={`kds-ticket kds-ticket--${level}${urgent ? ' kds-ticket--urgent' : ''}${selected ? ' kds-ticket--selected' : ''}${order.priority ? ' kds-ticket--rush' : ''}`}
      onClick={handleClick}
      aria-label={`${l10n.getString('kds-tap-to-advance-label', { number: order.display_number ?? 0 })} — ${level} SLA${urgent ? `, ${l10n.getString('kds-urgent-badge') || 'URGENT'}` : ''}${order.priority ? `, ${l10n.getString('kds-rush-badge') || 'RUSH'}` : ''}, ${display}`}
    >
      <div className="kds-ticket-header">
        <span className="kds-ticket-id-group">
          {showOrderId && <span className="kds-ticket-number">#{order.display_number}</span>}
          {showTableNumber && order.table_number && (
            <span className="kds-ticket-table">{order.table_number}</span>
          )}
        </span>
        <span className={`kds-ticket-time kds-ticket-time--${level}`}>{display}</span>
      </div>
      {order.priority && (
        <span className="kds-ticket-rush-badge">
          <Localized id="kds-rush-badge">RUSH</Localized>
        </span>
      )}
      {urgent && (
        <span className="kds-ticket-urgent-badge">
          <Localized id="kds-urgent-badge">URGENT</Localized>
        </span>
      )}

      {/* ── Course-grouped line items ─────────────────────────────── */}
      {courseGroups ? (
        <div className="kds-ticket-line-items">
          {courseGroups.map((group) => (
            <div key={group.course ?? '__other__'} className="kds-ticket-course-group">
              <span className="kds-ticket-course-header">{courseLabel(group.course)}</span>
              {group.items.map((item) => (
                <div key={item.id} className="kds-ticket-item-row">
                  <span className="kds-ticket-item-name">
                    {item.qty > 1 ? `${item.display_name} x${item.qty}` : item.display_name}
                  </span>
                  {item.modifiers.length > 0 && (
                    <span className="kds-ticket-modifiers">
                      {item.modifiers.map((mod, mi) => (
                        <span key={mi} className="kds-ticket-modifier-row">
                          {mod.choice}
                        </span>
                      ))}
                    </span>
                  )}
                </div>
              ))}
            </div>
          ))}
        </div>
      ) : (
        /* ── Fallback: flat items_summary (loading, old orders) ──── */
        <span className="kds-ticket-items">
          {lineItemsLoading
            ? l10n.getString('kds-course-loading') || 'Loading items...'
            : order.items_summary}
        </span>
      )}

      {order.notes && <span className="kds-ticket-notes">{order.notes}</span>}
      {editing && (
        <div
          className="kds-ticket-edit"
          onClick={(e) => e.stopPropagation()}
          onKeyDown={(e) => { if (e.key === 'Escape') handleCancelEdit(); }}
          role="presentation"
          tabIndex={-1}
        >
          <input
            ref={inputRef}
            className="kds-ticket-edit-input"
            type="text"
            value={editSummary}
            onChange={(e) => setEditSummary(e.target.value)}
            onKeyDown={handleKeyDown}
            aria-label={l10n.getString('kds-edit-items-aria') || 'Edit items'}
          />
          <div className="kds-ticket-edit-row">
            <label className="kds-ticket-edit-label">
              <Localized id="kds-edit-count-label">Count</Localized>:
              <input
                className="kds-ticket-edit-count"
                type="number"
                min={1}
                value={editCount}
                onChange={(e) => setEditCount(e.target.value)}
                onKeyDown={handleKeyDown}
                aria-label={l10n.getString('kds-edit-count-aria') || 'Item count'}
              />
            </label>
            <div className="kds-ticket-edit-actions">
              <button
                className="kds-ticket-edit-save"
                onClick={handleSaveEdit}
                disabled={!editSummary.trim() || parseInt(editCount, 10) <= 0}
                aria-label={l10n.getString('kds-edit-save-aria') || 'Save items'}
              >
                <Localized id="kds-edit-save">Save</Localized>
              </button>
              <button
                className="kds-ticket-edit-cancel"
                onClick={handleCancelEdit}
                aria-label={l10n.getString('kds-edit-cancel-aria') || 'Cancel edit'}
              >
                <Localized id="kds-edit-cancel">Cancel</Localized>
              </button>
            </div>
          </div>
        </div>
      )}
      <span className="kds-ticket-count">
        <Localized id="kds-items" vars={{ count: order.item_count }}>
          {`${order.item_count} items`}
        </Localized>
      </span>
      {onSaveItems && !editing && (
        <button
          className="kds-ticket-edit-btn"
          onClick={startEditing}
          aria-label={l10n.getString('kds-edit-items-btn-aria') || 'Edit ticket items'}
        >
          <Localized id="kds-edit-items-btn">Edit Items</Localized>
        </button>
      )}
    </button>
  );
});
