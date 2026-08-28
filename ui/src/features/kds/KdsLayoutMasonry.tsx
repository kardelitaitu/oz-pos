import { useMemo } from 'react';
import { KdsTicketCard } from '@/features/kds/components/KdsTicketCard';
import type { KdsLayoutProps } from './KdsScreen';
import type { KdsOrder } from '@/api/kds';

/**
 * KdsLayoutMasonry — the single KDS view (design-language prototype:
 * dev/kds-prototype.html).
 *
 * All open orders flow into N equal-width columns (a JS masonry: cards
 * are distributed round-robin so each column stays roughly balanced),
 * instead of the previous kanban/focus/metro layout switching. There is
 * deliberately no per-status grouping — kitchen awareness comes from the
 * card's SLA colour, status pill, and footer actions, not columns.
 *
 * Column count is responsive: 3 on wide kitchen displays, dropping to
 * 2 / 1 as the viewport narrows.
 */
export function KdsLayoutMasonry({
  orders,
  onAdvance,
  showOrderId,
  showTableNumber,
  selectedOrderId,
  onSaveItems,
  sessionToken,
  onAdvanceItem,
  onAddItems,
  newOrderIds,
}: KdsLayoutProps) {
  // Distribute orders round-robin across the column count so no single
  // column grows unboundedly as new tickets arrive.
  const columns = useMemo(() => {
    const count = 3;
    const cols: KdsOrder[][] = Array.from({ length: count }, () => []);
    orders.forEach((o, i) => {
      cols[i % count]!.push(o);
    });
    return cols;
  }, [orders]);

  if (orders.length === 0) {
    return (
      <div className="kds-main empty">
        <p className="kds-empty" role="status">No orders yet</p>
      </div>
    );
  }

  return (
    <div className="kds-main">
      {columns.map((col, ci) => (
        <div key={ci} className="kds-col">
          {col.map((order) => (
            <KdsTicketCard
              key={order.id}
              order={order}
              onAdvance={onAdvance}
              showOrderId={showOrderId}
              showTableNumber={showTableNumber}
              selected={selectedOrderId === order.id}
              sessionToken={sessionToken}
              isNew={newOrderIds.has(order.id)}
              {...(onSaveItems ? { onSaveItems } : {})}
              {...(onAdvanceItem ? { onAdvanceItem } : {})}
              {...(onAddItems ? { onAddItems } : {})}
            />
          ))}
        </div>
      ))}
    </div>
  );
}
