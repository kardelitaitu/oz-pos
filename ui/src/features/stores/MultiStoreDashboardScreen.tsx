import { useState, useEffect, useCallback } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import { listStores, setPrimaryStore, deleteStore, type StoreProfile } from '@/api/stores';
import { listTerminals, type TerminalDto } from '@/api/terminals';
import { Button } from '@/components/Button';
import { Card } from '@/components/Card';
import TerminalStatusPanel from './TerminalStatusPanel';
import './MultiStoreDashboardScreen.css';

const ONLINE_THRESHOLD_MS = 5 * 60 * 1000;

function isOnline(lastSeenAt: string | null): boolean {
  if (!lastSeenAt) return false;
  return Date.now() - new Date(lastSeenAt).getTime() < ONLINE_THRESHOLD_MS;
}

export default function MultiStoreDashboardScreen() {
  const { l10n } = useLocalization();
  const [stores, setStores] = useState<StoreProfile[]>([]);
  const [terminals, setTerminals] = useState<TerminalDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [deletingId, setDeletingId] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [storeData, termData] = await Promise.all([
        listStores(),
        listTerminals(),
      ]);
      setStores(storeData);
      setTerminals(termData);
    } catch {
      setError('Failed to load data');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { load(); }, [load]);

  const handleSetPrimary = useCallback(async (id: string) => {
    try {
      await setPrimaryStore(id);
      setStores((prev) =>
        prev.map((s) => ({ ...s, is_primary: s.id === id })),
      );
    } catch {
      // silently fail
    }
  }, []);

  const handleDelete = useCallback(async (id: string) => {
    setDeletingId(id);
    try {
      await deleteStore(id);
      setStores((prev) => prev.filter((s) => s.id !== id));
    } catch {
      // silently fail
    } finally {
      setDeletingId(null);
    }
  }, []);

  const activeTerminals = terminals.filter((t) => t.isActive).length;
  const onlineTerminals = terminals.filter((t) => isOnline(t.lastSeenAt)).length;

  const getTerminalCount = useCallback(
    (_storeId: string) => {
      return terminals.length;
    },
    [terminals],
  );

  return (
    <div className="multi-store-dashboard">
      <div className="multi-store-dashboard-header">
        <Localized id="multi-store-dashboard-title">
          <h1 className="multi-store-dashboard-title">Multi-Store Dashboard</h1>
        </Localized>
      </div>

      {loading ? (
        <p className="multi-store-dashboard-loading"><Localized id="multi-store-dashboard-loading">Loading dashboard…</Localized></p>
      ) : error ? (
        <Card shadow="sm">
          <div className="multi-store-dashboard-error">
            <p>{error}</p>
            <Button variant="secondary" onClick={load}><Localized id="retry">Retry</Localized></Button>
          </div>
        </Card>
      ) : (
        <>
          {/* ── Stat cards ────────────────────────────────────── */}
          <div className="multi-store-stat-grid">
            <div className="multi-store-stat-card">
              <span className="multi-store-stat-value">{stores.length}</span>
              <span className="multi-store-stat-label"><Localized id="multi-store-stat-total-stores">Total Stores</Localized></span>
            </div>
            <div className="multi-store-stat-card">
              <span className="multi-store-stat-value">{activeTerminals}</span>
              <span className="multi-store-stat-label"><Localized id="multi-store-stat-active-terminals">Active Terminals</Localized></span>
            </div>
            <div className="multi-store-stat-card">
              <span className="multi-store-stat-value">{onlineTerminals}</span>
              <span className="multi-store-stat-label"><Localized id="multi-store-stat-online-terminals">Online Terminals</Localized></span>
            </div>
            <div className="multi-store-stat-card">
              <span className="multi-store-stat-value">{terminals.length}</span>
              <span className="multi-store-stat-label"><Localized id="multi-store-stat-total-terminals">Total Terminals</Localized></span>
            </div>
          </div>

          {/* ── Store cards ───────────────────────────────────── */}
          <section aria-label={l10n.getString('multi-store-section-stores-overview')}>
            <h2 className="multi-store-section-title"><Localized id="multi-store-section-stores">Stores</Localized></h2>
            <div className="multi-store-card-grid">
              {stores.map((store) => {
                const tc = getTerminalCount(store.id);
                return (
                  <Card
                    key={store.id}
                    shadow={store.is_primary ? 'md' : 'sm'}
                    padding="md"
                    className={`multi-store-card ${store.is_primary ? 'multi-store-card--primary' : ''}`}
                    header={
                      <div className="multi-store-card-header">
                        <span className="multi-store-card-name">{store.name}</span>
                        {store.is_primary && (
                          <span className="multi-store-card-badge"><Localized id="multi-store-badge-primary">Primary</Localized></span>
                        )}
                      </div>
                    }
                    footer={
                      <div className="multi-store-card-actions">
                        {!store.is_primary && (
                          <>
                            <Button
                              variant="secondary"
                              size="sm"
                              onClick={() => handleSetPrimary(store.id)}
                              aria-label={l10n.getString('multi-store-btn-set-primary-label', { name: store.name })}
                            >
                              <Localized id="multi-store-btn-set-primary">Set as Primary</Localized>
                            </Button>
                            <Button
                              variant="danger"
                              size="sm"
                              loading={deletingId === store.id}
                              onClick={() => handleDelete(store.id)}
                              aria-label={l10n.getString('multi-store-btn-delete-label', { name: store.name })}
                            >
                              <Localized id="multi-store-btn-delete">Delete</Localized>
                            </Button>
                          </>
                        )}
                      </div>
                    }
                  >
                    <div className="multi-store-card-body">
                      {store.address && (
                        <div className="multi-store-card-row">
                          <span className="multi-store-card-label"><Localized id="multi-store-label-address">Address</Localized></span>
                          <span className="multi-store-card-value">{store.address}</span>
                        </div>
                      )}
                      {store.tax_id && (
                        <div className="multi-store-card-row">
                          <span className="multi-store-card-label"><Localized id="multi-store-label-tax-id">Tax ID</Localized></span>
                          <span className="multi-store-card-value">{store.tax_id}</span>
                        </div>
                      )}
                      <div className="multi-store-card-row">
                          <span className="multi-store-card-label"><Localized id="multi-store-label-currency">Currency</Localized></span>
                        <span className="multi-store-card-value">{store.currency}</span>
                      </div>
                      <div className="multi-store-card-row">
                          <span className="multi-store-card-label"><Localized id="multi-store-label-timezone">Timezone</Localized></span>
                        <span className="multi-store-card-value">{store.timezone}</span>
                      </div>
                      <div className="multi-store-card-row">
                          <span className="multi-store-card-label"><Localized id="multi-store-label-terminals">Terminals</Localized></span>
                        <span className="multi-store-card-value">{tc}</span>
                      </div>
                    </div>
                  </Card>
                );
              })}
            </div>
          </section>

          {/* ── Terminal Status Panel ─────────────────────────── */}
          <section aria-label={l10n.getString('multi-store-section-terminal-status')} className="multi-store-terminal-section">
            <TerminalStatusPanel refreshTrigger={0} />
          </section>
        </>
      )}
    </div>
  );
}
