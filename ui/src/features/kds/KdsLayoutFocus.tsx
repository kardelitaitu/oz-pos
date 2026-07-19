import { useState, useMemo } from 'react';
import { Localized } from '@fluent/react';
import { KdsTicketCard } from '@/features/kds/components/KdsTicketCard';
import type { KdsOrder } from '@/api/kds';
import type { KdsLayoutProps } from './KdsScreen';
import './KdsLayoutFocus.css';

type StatusFilter = 'all' | 'pending' | 'preparing' | 'ready';

const FILTERS: { key: StatusFilter; label: string }[] = [
  { key: 'all', label: 'kds-filter-all' },
  { key: 'pending', label: 'kds-pending' },
  { key: 'preparing', label: 'kds-preparing' },
  { key: 'ready', label: 'kds-ready' },
];

function slaWeight(order: KdsOrder): number {
  const age = Date.now() - new Date(order.received_at).getTime();
  return age;
}

export function KdsLayoutFocus({ orders, onAdvance, showOrderId, showTableNumber }: KdsLayoutProps) {
  const [filter, setFilter] = useState<StatusFilter>('all');

  const filtered = useMemo(() => {
    const statusFiltered = filter === 'all'
      ? orders
      : orders.filter((o) => o.status === filter);
    return [...statusFiltered].sort((a, b) => slaWeight(b) - slaWeight(a));
  }, [orders, filter]);

  const counts = useMemo(() => {
    const c = { all: orders.length, pending: 0, preparing: 0, ready: 0 };
    for (const o of orders) {
      if (o.status in c) c[o.status as keyof typeof c]++;
    }
    return c;
  }, [orders]);

  return (
    <div className="kds-focus">
      <div className="kds-focus-filters">
        {FILTERS.map(({ key, label }) => (
          <button
            key={key}
            className={`kds-focus-filter-btn ${filter === key ? 'kds-focus-filter-btn--active' : ''}`}
            onClick={() => setFilter(key)}
          >
            <Localized id={label}>{key}</Localized>
            <span className="kds-focus-filter-count">{counts[key]}</span>
          </button>
        ))}
      </div>
      <div className="kds-focus-tickets">
        {filtered.length === 0 ? (
          <p className="kds-empty"><Localized id="kds-no-orders">No orders yet</Localized></p>
        ) : (
          filtered.map((order) => (
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
  );
}
