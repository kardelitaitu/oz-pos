import { useEffect, useRef, useState } from 'react';
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

/** Animation modifier: 'up', 'down', or '' (idle). */
type AnimDir = 'up' | 'down' | '';

/**
 * Tracks count changes and returns an animation direction for the
 * duration of the bounce animation.
 */
function useCountAnim(count: number): AnimDir {
  const prevRef = useRef(count);
  const [anim, setAnim] = useState<AnimDir>('');

  useEffect(() => {
    const prev = prevRef.current;
    if (count !== prev) {
      const dir: AnimDir = count > prev ? 'up' : 'down';
      setAnim(dir);
      // Clear the animation class after 300ms (matches CSS duration).
      const timer = setTimeout(() => setAnim(''), 300);
      prevRef.current = count;
      return () => clearTimeout(timer);
    }
    prevRef.current = count;
  }, [count]);

  return anim;
}

export function KdsLayoutKanban({ orders, onAdvance, showOrderId, showTableNumber }: KdsLayoutProps) {
  const grouped = (status: ColumnStatus) =>
    orders.filter((o) => o.status === status);

  // Hoist useCountAnim calls to top level (Rules of Hooks compliance).
  const anims: Record<ColumnStatus, AnimDir> = {
    pending: useCountAnim(grouped('pending').length),
    preparing: useCountAnim(grouped('preparing').length),
    ready: useCountAnim(grouped('ready').length),
  };

  return (
    <div className="kds-columns">
      {STATUS_ORDER.map((status) => {
        const count = grouped(status).length;
        const anim = anims[status];
        const countClass = `kds-column-count${anim ? ` kds-column-count--${anim}` : ''}`;

        return (
          <div key={status} className={`kds-column kds-column--${status}`}>
            <h2 className="kds-column-title">
              <Localized id={STATUS_LABELS[status]}>
                <span>{status}</span>
              </Localized>
              <span className={countClass}>{count}</span>
            </h2>
            <div className="kds-tickets">
              {count === 0 ? (
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
        );
      })}
    </div>
  );
}
