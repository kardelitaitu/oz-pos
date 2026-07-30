import { Localized } from '@fluent/react';
import { KdsTicketCard } from '@/features/kds/components/KdsTicketCard';
import type { KdsLayoutProps } from './KdsScreen';
import './KdsLayoutMetro.css';

export function KdsLayoutMetro({ orders, onAdvance, showOrderId, showTableNumber, selectedOrderId, onSaveItems, sessionToken, onAdvanceItem, onAddItems }: KdsLayoutProps) {
  return (
    <div className="kds-metro">
      {orders.length === 0 ? (
        <p className="kds-empty"><Localized id="kds-no-orders">No orders yet</Localized></p>
      ) : (
        orders.map((order) => (
          <KdsTicketCard
            key={order.id}
            order={order}
            onAdvance={onAdvance}
            showOrderId={showOrderId}
            showTableNumber={showTableNumber}
            selected={selectedOrderId === order.id}
            sessionToken={sessionToken}
            {...(onSaveItems ? { onSaveItems } : {})}
            {...(onAdvanceItem ? { onAdvanceItem } : {})}
            {...(onAddItems ? { onAddItems } : {})}
          />
        ))
      )}
    </div>
  );
}
