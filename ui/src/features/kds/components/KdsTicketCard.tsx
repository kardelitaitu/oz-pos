import { useEffect, useRef } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import { useTicketSla } from '@/features/kds/hooks/useTicketSla';
import { useSound } from '@/frontend/shared/useSound';
import type { KdsOrder, KdsStatus } from '@/api/kds';

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
}

const STATUS_ORDER: KdsStatus[] = ['pending', 'preparing', 'ready', 'served'];

/**
 * KdsTicketCard renders a single KDS ticket with SLA aging indicators
 * and plays an audio alert when the ticket enters the red threshold.
 */
export function KdsTicketCard({ order, onAdvance, showOrderId = true, showTableNumber = true }: KdsTicketCardProps) {
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

  const handleClick = () => {
    const currentIdx = STATUS_ORDER.indexOf(order.status as KdsStatus);
    if (currentIdx >= 0 && currentIdx < STATUS_ORDER.length - 1) {
      onAdvance(order);
    }
  };

  return (
    <button
      className={`kds-ticket kds-ticket--${level}${urgent ? ' kds-ticket--urgent' : ''}`}
      onClick={handleClick}
      aria-label={`${l10n.getString('kds-tap-to-advance-label', { number: order.display_number ?? 0 })} — ${level} SLA${urgent ? ', URGENT' : ''}, ${display}`}
    >
      <div className="kds-ticket-header">
        <span className="kds-ticket-id-group">
          {showOrderId && <span className="kds-ticket-number">#{order.display_number}</span>}
          {showTableNumber && !!((order as unknown as Record<string, unknown>)['table_number']) && (
            <span className="kds-ticket-table">{(order as unknown as Record<string, unknown>)['table_number'] as string}</span>
          )}
        </span>
        <span className={`kds-ticket-time kds-ticket-time--${level}`}>{display}</span>
      </div>
      {urgent && (
        <span className="kds-ticket-urgent-badge">
          <Localized id="kds-urgent-badge">URGENT</Localized>
        </span>
      )}
      <span className="kds-ticket-items">{order.items_summary}</span>
      {order.notes && <span className="kds-ticket-notes">{order.notes}</span>}
      <span className="kds-ticket-count">
        <Localized id="kds-items" vars={{ count: order.item_count }}>
          {`${order.item_count} items`}
        </Localized>
      </span>
    </button>
  );
}
