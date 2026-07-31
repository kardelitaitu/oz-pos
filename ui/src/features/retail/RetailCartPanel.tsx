import { useState, useRef, useEffect, type CSSProperties } from 'react';
import { useLocalization, Localized } from '@fluent/react';
import { formatMoney, type Money, type LineId, type Sku, type CourseId } from '@/types/domain';
import { COURSES, courseLabel, courseEmoji } from '@/types/domain';
import type { CartLine } from '@/types/domain';
import type { CustomerDto } from '@/api/customers';
import { clampRetailCartWidth } from './RetailCartPanel.constants';

// ── Grouped prop interfaces ────────────────────────────────────────

export interface CartTotalsData {
  subtotal: Money | null;
  total: Money | null;
  discountPercent: number;
  discountAmount: Money | null;
  cartTax: number;
}

export interface CartLineActions {
  onRemoveLine: (id: string, line: { sku: Sku; name: string; category: string; unit_price: Money; qty: number }) => void;
  onIncreaseQty: (line: { sku: string; id: LineId; qty: number }) => void;
  onUpdateQty: (lineId: LineId, qty: number) => void;
  onSerialChange: (lineId: string, serial: string) => void;
  onSetOverrideTarget: (target: { id: LineId; name: string; unit_price: Money } | null) => void;
  /** Assign a course to a cart line (restaurant coursing). */
  onAssignCourse: (lineId: LineId, courseId: CourseId) => void;
  /** Open modifier editor for a cart line. */
  onEditModifiers: (line: CartLine) => void;
}

export interface CartPanelActions {
  onPay: () => void;
  onShowDiscount: () => void;
  onHoldResume: () => void;
  onRequestClear: () => void;
  onShowCreditList: () => void;
  onLoadCreditSales: () => void;
}

export interface RetailCartPanelProps {
  // Cart data
  lines: CartLine[];
  /** Whether to show course chips (restaurant mode or product has category mapping). */
  showCourseSelector?: boolean;
  lineCount: number;
  selectedCustomer: CustomerDto | null;
  totals: CartTotalsData;

  // Cart state
  retailCartWidth: number;
  serialNumbers: Record<string, string>;
  trackSerialMap: Record<string, boolean>;
  overrideTarget: { id: LineId; name: string; unit_price: Money } | null;
  undoStack: { sku: Sku; name: string; category: string; unit_price: Money; qty: number }[];
  undoBarExit: { shouldRender: boolean; exiting: boolean; requestClose: () => void };

  // Feature flags
  isSerialTracking: boolean;
  isManager: boolean;
  activeShift: boolean;
  heldCartId: string | null;

  // Resize
  cartWidthMin: number;
  cartWidthMaxCap: number;
  onResizeWidth: React.Dispatch<React.SetStateAction<number>>;
  onStartResize: (e: React.MouseEvent) => void;
  /** Spread onto the cart container div for swipe-to-pay gesture support */
  cartSwipe: Record<string, unknown>;

  // Actions
  lineActions: CartLineActions;
  panelActions: CartPanelActions;
  onUndoRemove: () => void;
  onDismissUndo: () => void;
  onEnsureCart: (currency: string) => void;
}

/** Cart panel — cart lines table, undo bar, totals, action buttons, and the resize handle. */
export default function RetailCartPanel({
  lines,
  lineCount,
  selectedCustomer,
  totals,
  retailCartWidth,
  serialNumbers,
  trackSerialMap,
  undoStack,
  undoBarExit,
  isSerialTracking,
  isManager,
  activeShift,
  heldCartId,
  cartWidthMin,
  cartWidthMaxCap,
  onResizeWidth,
  onStartResize,
  cartSwipe,
  lineActions,
  panelActions,
  onUndoRemove,
  onDismissUndo,
  onEnsureCart,
  showCourseSelector = false,
}: RetailCartPanelProps) {
  const { l10n } = useLocalization();

  // ── Course dropdown state ────────────────────────────────────
  const [courseDropdownLine, setCourseDropdownLine] = useState<LineId | null>(null);
  const courseDropdownGroupRef = useRef<HTMLSpanElement>(null);

  // Close course dropdown when clicking outside
  useEffect(() => {
    if (!courseDropdownLine) return;
    const handler = (e: MouseEvent) => {
      if (courseDropdownGroupRef.current && !courseDropdownGroupRef.current.contains(e.target as Node)) {
        setCourseDropdownLine(null);
      }
    };
    // Use mousedown so it fires before the toggle button's onClick on re-click
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, [courseDropdownLine]);

  return (
    <>
      {/* eslint-disable jsx-a11y/no-noninteractive-element-interactions, jsx-a11y/no-noninteractive-tabindex -- role=separator makes this interactive per ARIA spec */}
      <div
        className="retail-resize-handle"
        onMouseDown={onStartResize}
        role="separator"
        aria-orientation="vertical"
        aria-valuenow={retailCartWidth}
        aria-valuemin={cartWidthMin}
        aria-valuemax={clampRetailCartWidth(cartWidthMaxCap, window.innerWidth)}
        aria-label={l10n.getString('retail-resize-handle-aria') || 'Resize cart panel'}
        tabIndex={0}
        onKeyDown={(e) => {
          if (e.key === 'ArrowLeft') {
            onResizeWidth((w) => clampRetailCartWidth(w - 20, window.innerWidth));
            e.preventDefault();
          } else if (e.key === 'ArrowRight') {
            onResizeWidth((w) => clampRetailCartWidth(w + 20, window.innerWidth));
            e.preventDefault();
          }
        }}
      />
      {/* eslint-enable jsx-a11y/no-noninteractive-element-interactions, jsx-a11y/no-noninteractive-tabindex */}

      {/* Right: cart */}
      <div className="retail-cart" style={{ width: retailCartWidth } as CSSProperties} {...cartSwipe}>
        <div className="retail-cart-header">
          <span>{l10n.getString('cart-title')}</span>
          <span>{l10n.getString('retail-cart-items', { count: lineCount }) || `${lineCount} item${lineCount !== 1 ? 's' : ''}`}</span>
        </div>
        {lines.length === 0 ? (
          <div className="retail-cart-empty">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
              <path d="M6 2 4 6v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V6l-2-4H6z" />
              <path d="M4 6h16" />
              <path d="M9 10V8a3 3 0 0 1 6 0v2" />
            </svg>
            <span>{l10n.getString('pos-cart-empty')}</span>
          </div>
        ) : (
          <>
            <div className="retail-cart-table">
              <table className="retail-cart-table-inner">
                <thead>
                  <tr>
                    <th className="retail-cart-th-num">{l10n.getString('retail-cart-header-col')}</th>
                    <th>{l10n.getString('retail-cart-header-item')}</th>
                    <th className="retail-cart-th-qty">{l10n.getString('retail-cart-header-qty')}</th>
                    <th className="retail-cart-th-price">{l10n.getString('retail-cart-header-price')}</th>
                    <th className="retail-cart-th-subtotal">{l10n.getString('retail-cart-header-subtotal')}</th>
                    <th className="retail-cart-th-actions"></th>
                  </tr>
                </thead>
                <tbody>{lines.map((line, idx) => (
                    <tr key={line.id}>
                      <td className="retail-cart-line-sku">{idx + 1}</td>
                      <td>
                        <div className="retail-cart-line-name">
                          {line.name ?? line.sku}
                          {/* ── Course chip ──────────── */}
                          {showCourseSelector && (
                            <span className="retail-cart-course-chip-group">
                              <button
                                type="button"
                                className={`retail-cart-course-chip${line.courseId ? ' retail-cart-course-chip--set' : ''}`}
                                onClick={(e) => { e.stopPropagation(); setCourseDropdownLine(courseDropdownLine === line.id ? null : line.id); }}
                                aria-label={l10n.getString('retail-cart-course-aria', { name: line.name ?? line.sku }) || `Course for ${line.name ?? line.sku}`}
                                title={line.courseId ? `${courseEmoji(line.courseId)} ${courseLabel(line.courseId)}` : 'Set course'}
                              >
                                {line.courseId ? `${courseEmoji(line.courseId)} ${courseLabel(line.courseId)}` : '🍽️ Course'}
                              </button>
                              {/* ── Course dropdown ──── */}
                              {courseDropdownLine === line.id && (
                                <span className="retail-cart-course-dropdown" role="listbox" aria-label="Select course" ref={courseDropdownGroupRef}>
                                  <button
                                    type="button"
                                    className={`retail-cart-course-option${!line.courseId ? ' retail-cart-course-option--active' : ''}`}
                                    onClick={() => { lineActions.onAssignCourse(line.id as LineId, '' as CourseId); setCourseDropdownLine(null); }}
                                    role="option"
                                    aria-selected={!line.courseId}
                                  >
                                    None
                                  </button>
                                  {COURSES.map((c) => (
                                    <button
                                      key={c.id}
                                      type="button"
                                      className={`retail-cart-course-option${line.courseId === c.id ? ' retail-cart-course-option--active' : ''}`}
                                      onClick={() => { lineActions.onAssignCourse(line.id as LineId, c.id); setCourseDropdownLine(null); }}
                                      role="option"
                                      aria-selected={line.courseId === c.id}
                                    >
                                      {c.emoji} {c.label}
                                    </button>
                                  ))}
                                </span>
                              )}
                            </span>
                          )}
                          {/* ── Modifier badge ──────── */}
                          {line.modifiers && line.modifiers.length > 0 && (
                            <span className="retail-cart-modifier-badge">
                              +{line.modifiers.length}
                            </span>
                          )}
                        </div>
                        {/* ── Modifier names line ──── */}
                        {line.modifiers && line.modifiers.length > 0 && (
                          <div className="retail-cart-line-modifiers">
                            {line.modifiers.map((m) => m.modifierName).join(', ')}
                          </div>
                        )}
                        {isSerialTracking && trackSerialMap[line.sku] && (
                          <input
                            type="text"
                            className="retail-cart-serial-input"
                            value={serialNumbers[line.id] ?? ''}
                            onChange={(e) => lineActions.onSerialChange(line.id, e.target.value)}
                            placeholder={l10n.getString('retail-serial-placeholder') || 'Serial #'}
                            aria-label={l10n.getString('retail-serial-aria', { name: line.name ?? line.sku }) || `Serial number for ${line.name ?? line.sku}`}
                          />
                        )}
                      </td>
                      <td>
                        <span className="retail-cart-line-qty">
                          <button
                            className="retail-cart-qty-btn"
                            onClick={() => {
                              const newQty = line.qty - 1;
                              if (newQty <= 0) {
                                lineActions.onRemoveLine(line.id, { sku: line.sku, name: line.name ?? '', category: line.category ?? '', unit_price: line.unit_price, qty: line.qty });
                              } else {
                                lineActions.onUpdateQty(line.id, newQty);
                              }
                            }}
                            aria-label={l10n.getString('retail-cart-qty-decrease-aria') || `Decrease quantity of ${line.sku}`}
                          >
                            &minus;
                          </button>
                          <span className="retail-cart-qty-value">{line.qty}</span>
                          <button
                            className="retail-cart-qty-btn"
                            onClick={() => lineActions.onIncreaseQty(line)}
                            aria-label={l10n.getString('retail-cart-qty-increase-aria') || `Increase quantity of ${line.sku}`}
                          >
                            +
                          </button>
                        </span>
                        {/* ── Edit modifiers ──────── */}
                        {showCourseSelector && (
                          <button
                            type="button"
                            className="retail-cart-modifier-btn"
                            onClick={() => lineActions.onEditModifiers(line)}
                            aria-label={l10n.getString('retail-cart-modifier-aria', { name: line.name ?? line.sku }) || `Modifiers for ${line.name ?? line.sku}`}
                          >
                            <Localized id="retail-cart-modifier-btn">
                              <span>Modifiers</span>
                            </Localized>
                          </button>
                        )}
                      </td>
                      <td className="retail-cart-line-unit">
                        {formatMoney(line.unit_price)}
                        {isManager && (
                          <button
                            type="button"
                            className="retail-cart-line-override"
                            onClick={() => {
                              lineActions.onSetOverrideTarget({ id: line.id as LineId, name: line.name ?? line.sku, unit_price: line.unit_price });
                              onEnsureCart(line.unit_price.currency);
                            }}
                            aria-label={l10n.getString('retail-override-aria', { name: line.name ?? line.sku }) || `Override price for ${line.name ?? line.sku}`}
                          >
                            <Localized id="retail-override-btn"><span>Override</span></Localized>
                          </button>
                        )}
                      </td>
                      <td className="retail-cart-line-subtotal">{formatMoney({ minor_units: line.unit_price.minor_units * line.qty, currency: line.unit_price.currency })}</td>
                        <td>
                          <button type="button" className="retail-cart-remove-btn" onClick={() => lineActions.onRemoveLine(line.id, { sku: line.sku, name: line.name ?? '', category: line.category ?? '', unit_price: line.unit_price, qty: line.qty })} aria-label={l10n.getString('retail-cart-remove-aria') || `Remove ${line.sku} from cart`}>
                            &times;
                          </button>
                      </td>
                    </tr>
                  ))}
</tbody>
              </table>
            </div>

            {/* ── Undo bar ───────────── */}
            {undoBarExit.shouldRender && (
              <div
                className={`retail-undo-bar${undoBarExit.exiting ? ' retail-undo-bar--exiting' : ''}`}
                role="status"
                aria-live="polite"
              >
                <span className="retail-undo-bar-label">{l10n.getString('retail-undo-items-removed', { count: undoStack.length }) || `${undoStack.length} item${undoStack.length > 1 ? 's' : ''} removed`}</span>
                <button type="button" className="retail-undo-bar-btn" onClick={onUndoRemove}>{l10n.getString('pos-cart-undo')}</button>
                <button type="button" className="retail-undo-bar-dismiss" onClick={onDismissUndo} aria-label={l10n.getString('pos-cart-undo-dismiss-aria')}>&times;</button>
              </div>
            )}

            <div className="retail-cart-totals">
              <div className="retail-total-row">
                <span>{l10n.getString('pos-cart-subtotal')}</span>
                <span>{totals.subtotal ? formatMoney(totals.subtotal) : '—'}</span>
              </div>
              {totals.discountPercent > 0 && totals.discountAmount && (
                <div className="retail-total-row">
                  <span>{l10n.getString('retail-total-discount', { percent: totals.discountPercent }) || `Discount ${totals.discountPercent}%`}</span>
                  <span className="retail-total-discount">&minus;{formatMoney(totals.discountAmount)}</span>
                </div>
              )}
              {totals.cartTax > 0 && (
                <div className="retail-total-row">
                  <span>{l10n.getString('retail-total-tax')}</span>
                  <span>{formatMoney({ minor_units: totals.cartTax, currency: totals.subtotal?.currency ?? 'IDR' })}</span>
                </div>
              )}
              <div className="retail-total-row retail-total-row--grand" aria-live="polite" aria-atomic="true">
                <span>{l10n.getString('cart-total-label')}</span>
                <span>{totals.total ? formatMoney(totals.total) : '—'}</span>
              </div>
            </div>
            {selectedCustomer && (
              <div className="retail-customer-badge">
                <svg viewBox="0 0 20 20" fill="currentColor" width="14" height="14" aria-hidden="true">
                  <path d="M10 10a4 4 0 100-8 4 4 0 000 8zm-7 8a7 7 0 1114 0H3z" />
                </svg>
                <span>{selectedCustomer.name}</span>
              </div>
            )}
            <div className="retail-cart-actions">
              <button
                type="button"
                className="retail-cart-action-btn retail-cart-action-btn--pay"
                data-testid="pay-btn"
                onClick={panelActions.onPay}
                disabled={lines.length === 0 || !activeShift}
                aria-label={l10n.getString('sale-pay-button')}
              >
                {l10n.getString('sale-pay-button')}
              </button>
              <button
                type="button"
                className="retail-cart-action-btn retail-cart-action-btn--discount"
                onClick={panelActions.onShowDiscount}
                disabled={lines.length === 0}
                aria-label={l10n.getString('retail-discount-button')}
              >
                {l10n.getString('retail-discount-button')}
              </button>
              <button
                type="button"
                className="retail-cart-action-btn retail-cart-action-btn--hold"
                onClick={panelActions.onHoldResume}
                disabled={!heldCartId && lines.length === 0}
                aria-label={heldCartId ? (l10n.getString('retail-resume-button') || 'Resume') : (l10n.getString('pos-cart-hold') || 'Hold')}
              >
                {heldCartId ? (l10n.getString('retail-resume-button') || 'Resume') : (l10n.getString('pos-cart-hold') || 'Hold')}
              </button>
              <button
                type="button"
                className="retail-cart-action-btn retail-cart-action-btn--void"
                onClick={panelActions.onRequestClear}
                disabled={lines.length === 0}
                aria-label={l10n.getString('pos-cart-clear')}
              >
                {l10n.getString('pos-cart-clear')}
              </button>
            </div>
          </>
        )}
      </div>
    </>
  );
}
