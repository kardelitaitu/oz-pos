import { Localized } from '@fluent/react';
import { KdsTicketCard } from '@/features/kds/components/KdsTicketCard';
import type { KdsLayoutProps } from './KdsScreen';
import './KdsLayoutKanban.css';

const STATUS_ORDER = ['pending', 'preparing', 'ready'] as const;
type ColumnStatus = (typeof STATUS_ORDER)[number];

const STATUS_LABELS: Record<ColumnStatus, string> = {
  pending: 'kds-pending',
  preparing: 'kds-preparing',
  ready: 'kds-ready',
};

export function KdsLayoutKanban({ orders, onAdvance, showOrderId, showTableNumber }: KdsLayoutProps) {
  const grouped = (status: ColumnStatus) =>
    orders.filter((o) => o.status === status);

  return (
    <div className="kds-columns">
      {STATUS_ORDER.map((status) => (
        <div key={status} className={`kds-column kds-column--${status}`}>
          <h2 className="kds-column-title">
            <Localized id={STATUS_LABELS[status]}>
              <span>{status}</span>
            </Localized>
            <span className="kds-column-count">{grouped(status).length}</span>
          </h2>
          <div className="kds-tickets">
            {grouped(status).length === 0 ? (
              <p className="kds-empty"><Localized id="kds-no-orders">No orders yet</Localized></p>
            ) : (
              grouped(status).map((order) => (
                <KdsTicketCard
                  key={order.id}
                  order={order}
                  onAdvance={onAdvance}
                  showOrderId={showOrderId}
                  showTableNumber={showTableNumber}
                />
              ))
            )}
          </div>
        </div>
      ))}
    </div>
  );
}
