import { useState, useCallback, useEffect } from 'react';
import StockCountsScreen from './StockCountsScreen';
import StockCountForm from './StockCountForm';
import StockCountDetail from './StockCountDetail';
import StockCountHistory from './StockCountHistory';
import type { StockCountDto } from '@/api/inventoryCounts';

type View = 'list' | 'new' | 'detail' | 'history';

/** Stock counts feature router — orchestrates navigation between list, new, detail, and history views based on hash or local state. */
export default function StockCountsFlow() {
  const [view, setView] = useState<View>('list');
  const [selectedId, setSelectedId] = useState<string | null>(null);

  const handleCancel = useCallback(() => setView('list'), []);

  const handleCreated = useCallback((_count: StockCountDto) => {
    setView('list');
  }, []);

  const handleBack = useCallback(() => {
    setSelectedId(null);
    setView('list');
  }, []);

  // We need to pass callbacks to StockCountsScreen for navigation.
  // Since it's the default view, we render it with the standard routing.
  // For now, list->detail navigation uses window.location.hash.
  // We listen for hash changes.
  const [hash, setHash] = useState(() => window.location.hash);

  // Listen for hash changes in a useEffect with a cleanup so the
  // listener is removed on unmount (prevents a leak that would leave a
  // stale setHash closure attached to window forever, causing state
  // updates on an unmounted component).
  useEffect(() => {
    const handleHashChange = () => {
      setHash(window.location.hash);
    };
    window.addEventListener('hashchange', handleHashChange);
    return () => {
      window.removeEventListener('hashchange', handleHashChange);
    };
  }, []);

  // Parse hash for routing
  const hashMatch = hash.match(/^#stock-count-(.+)$/);
  const hashNew = hash === '#stock-count-new';

  if (hashNew || view === 'new') {
    return (
      <StockCountForm
        onCreated={(c) => {
          handleCreated(c);
          window.location.hash = '';
        }}
        onCancel={() => {
          handleCancel();
          window.location.hash = '';
        }}
      />
    );
  }

  if (hashMatch || (view === 'detail' && selectedId)) {
    const id = hashMatch?.[1] ?? selectedId!;
    return (
      <StockCountDetail
        countId={id}
        onBack={() => {
          handleBack();
          window.location.hash = '';
        }}
      />
    );
  }

  if (hash === '#stock-count-history' || view === 'history') {
    return <StockCountHistory />;
  }

  return <StockCountsScreen />;
}
