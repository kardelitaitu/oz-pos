import { useState, useCallback, useEffect, useRef } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import { requiredLocalized } from '@/frontend/shared';
import { animDuration } from '@/utils/animation';
import { listPromotions, listPromotionsScoped, type Promotion } from '@/api/promotions';
import { useFocusTrap } from '@/hooks/useFocusTrap';
import { l10nErrorMessage } from '@/utils/app-error';
import { evaluatePromotionEligibility } from './promotionEligibility';
import './PromotionsModal.css';

/** Props for the PromotionsModal — lets the cashier apply an eligible promotion to the current cart. */
export interface PromotionsModalProps {
  open: boolean;
  sessionToken: string;
  /** Cart subtotal in minor units — used for min-order eligibility. */
  subtotalMinor: number;
  /** Called with an eligible promotion the cashier picked. */
  onSelect: (promo: Promotion) => void;
  onClose: () => void;
}

/** Promotions picker — lists active promotions and applies an eligible one to the cart discount. */
export default function PromotionsModal({
  open,
  sessionToken,
  subtotalMinor,
  onSelect,
  onClose,
}: PromotionsModalProps) {
  const { l10n } = useLocalization();
  const ANIM_MS = animDuration(200);
  const [exiting, setExiting] = useState(false);
  const exitTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const panelRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    return () => {
      if (exitTimerRef.current !== null) {
        clearTimeout(exitTimerRef.current);
        exitTimerRef.current = null;
      }
    };
  }, []);

  const handleClose = useCallback(() => {
    setExiting(true);
    exitTimerRef.current = setTimeout(() => {
      setExiting(false);
      exitTimerRef.current = null;
      onClose();
    }, ANIM_MS);
  }, [onClose, ANIM_MS]);

  const [promos, setPromos] = useState<Promotion[] | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);

  useFocusTrap(panelRef, open, handleClose);

  useEffect(() => {
    if (!open) return;
    let cancelled = false;
    setPromos(null);
    setLoadError(null);
    const load = sessionToken
      ? listPromotionsScoped(sessionToken)
      : listPromotions();
    load
      .then((list) => {
        if (cancelled) return;
        // Only currently-active promotions are pickable.
        setPromos(list.filter((p) => p.active));
      })
      .catch((err) => {
        if (cancelled) return;
        setLoadError(l10nErrorMessage(err, l10n, 'pos-promotions-load-failed'));
      });
    return () => {
      cancelled = true;
    };
  }, [open, sessionToken, l10n]);

  if (!open) return null;

  const items = promos ? evaluatePromotionEligibility(promos, subtotalMinor) : null;

  return (
    <div
      className={`promo-picker-overlay${exiting ? ' promo-picker-overlay--exiting' : ''}`}
      role="dialog"
      aria-modal="true"
      aria-label={requiredLocalized(l10n, 'pos-promotions-dialog-aria')}
    >
      <div className={`promo-picker-modal${exiting ? ' promo-picker-modal--exiting' : ''}`} ref={panelRef}>
          <button
            type="button"
            className="promo-picker-close"
            onClick={handleClose}
            aria-label={requiredLocalized(l10n, 'pos-promotions-close-aria')}
          >
            &times;
          </button>

          <Localized id="pos-promotions-title">
            <h2 className="promo-picker-title">Promotions</h2>
          </Localized>

          {loadError && (
            <div className="promo-picker-error" role="alert">
              {loadError}
            </div>
          )}

          {!items && !loadError && (
            <div className="promo-picker-loading" role="status">
              {requiredLocalized(l10n, 'pos-promotions-loading')}
            </div>
          )}

          {items && items.length === 0 && (
            <p className="promo-picker-empty">{requiredLocalized(l10n, 'pos-promotions-empty')}</p>
          )}

          {items && items.length > 0 && (
            <ul className="promo-picker-list">
              {items.map(({ promo, kind }) => (
                <li key={promo.id} className="promo-picker-item">
                  <button
                    type="button"
                    className={`promo-picker-item-btn${kind === 'eligible' ? '' : ' promo-picker-item-btn--disabled'}`}
                    disabled={kind !== 'eligible'}
                    onClick={() => onSelect(promo)}
                    aria-label={
                      kind === 'eligible'
                        ? l10n.getString('pos-promotions-apply-aria', { name: promo.name })
                        : l10n.getString('pos-promotions-unavailable-aria', { name: promo.name })
                    }
                  >
                    <span className="promo-picker-item-name">{promo.name}</span>
                    <span className="promo-picker-item-value">
                      {promo.promo_type === 'percentage'
                        ? requiredLocalized(l10n, 'pos-promotions-value-percent', { value: String(promo.value_minor) })
                        : promo.promo_type === 'fixed_amount'
                          ? requiredLocalized(l10n, 'pos-promotions-value-fixed', { value: String(promo.value_minor) })
                          : requiredLocalized(l10n, 'pos-promotions-value-bxgy')}
                    </span>
                    <span className="promo-picker-item-hint">
                      {kind === 'eligible'
                        ? (promo.description || promo.name)
                        : kind === 'below-min-order'
                          ? requiredLocalized(l10n, 'pos-promotions-min-order', {
                              min: String(promo.min_order_minor),
                            })
                          : requiredLocalized(l10n, 'pos-promotions-not-applicable')}
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          )}
      </div>
    </div>
  );
}
