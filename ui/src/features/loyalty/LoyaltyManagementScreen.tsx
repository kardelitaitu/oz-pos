import { useState, useCallback, useEffect, Fragment } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import {
  listLoyaltyAccounts,
  listLoyaltyTiers,
  updateLoyaltyTier,
  type LoyaltyAccountWithDetails,
  type LoyaltyTier,
} from '@/api/loyalty';
import { listCustomers, type CustomerDto } from '@/api/customers';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import { Skeleton } from '@/components/Skeleton';
import './LoyaltyManagementScreen.css';

interface TierFormData {
  name: string;
  min_points: string;
  points_per_unit: string;
  earn_multiplier: string;
  colour: string;
}

/** Loyalty management screen — view loyalty accounts, manage tiers, points configuration, and earn multipliers. */
export default function LoyaltyManagementScreen() {
  const { l10n } = useLocalization();
  const [accounts, setAccounts] = useState<LoyaltyAccountWithDetails[]>([]);
  const [customers, setCustomers] = useState<CustomerDto[]>([]);
  const [tiers, setTiers] = useState<LoyaltyTier[]>([]);
  const [loading, setLoading] = useState(true);
  const [tierTab, setTierTab] = useState(false);

  const [selectedAccount, setSelectedAccount] = useState<string | null>(null);
  const [editingTier, setEditingTier] = useState<string | null>(null);
  const [tierForm, setTierForm] = useState<TierFormData>({
    name: '', min_points: '', points_per_unit: '', earn_multiplier: '', colour: '',
  });
  const [savingTier, setSavingTier] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const customerMap = new Map(customers.map((c) => [c.id, c.name]));

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [accs, custs, t] = await Promise.all([
        listLoyaltyAccounts(),
        listCustomers(),
        listLoyaltyTiers(),
      ]);
      setAccounts(accs);
      setCustomers(custs);
      setTiers(t);
    } catch {
      // IPC unavailable.
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { load(); }, [load]);

  const openEditTier = useCallback((tier: LoyaltyTier) => {
    setTierForm({
      name: tier.name,
      min_points: String(tier.min_points),
      points_per_unit: String(tier.points_per_unit),
      earn_multiplier: String(tier.earn_multiplier),
      colour: tier.colour,
    });
    setEditingTier(tier.id);
    setError(null);
  }, []);

  const handleSaveTier = useCallback(async () => {
    if (!editingTier) return;
    const minPts = parseInt(tierForm.min_points, 10);
    const ppu = parseInt(tierForm.points_per_unit, 10);
    const mult = parseFloat(tierForm.earn_multiplier);
    if (Number.isNaN(minPts) || Number.isNaN(ppu) || Number.isNaN(mult) || !tierForm.name.trim()) {
      setError(l10n.getString('loyalty-validation-error'));
      return;
    }
    setSavingTier(true);
    setError(null);
    try {
      const updated = await updateLoyaltyTier({
        id: editingTier,
        name: tierForm.name.trim(),
        min_points: minPts,
        points_per_unit: ppu,
        earn_multiplier: mult,
        colour: tierForm.colour,
        sort_order: tiers.find((t) => t.id === editingTier)?.sort_order ?? 0,
        created_at: tiers.find((t) => t.id === editingTier)?.created_at ?? '',
      });
      setTiers((prev) => prev.map((t) => (t.id === updated.id ? updated : t)));
      setEditingTier(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to save tier');
    } finally {
      setSavingTier(false);
    }
  }, [editingTier, tierForm, tiers, l10n]);

  return (
    <div className="loyalty-mgmt">
      <div className="loyalty-mgmt-header">
        <Localized id="loyalty-title">
          <h1 className="loyalty-mgmt-title">Loyalty</h1>
        </Localized>
        <div className="loyalty-mgmt-tabs">
          <Button variant={tierTab ? 'ghost' : 'primary'} onClick={() => setTierTab(false)}>
            <Localized id="loyalty-accounts">Accounts</Localized>
          </Button>
          <Button variant={tierTab ? 'primary' : 'ghost'} onClick={() => setTierTab(true)}>
            <Localized id="loyalty-tiers">Tiers</Localized>
          </Button>
        </div>
      </div>

      {loading ? (
        <div className="loyalty-mgmt-loading-skeleton" aria-hidden="true">
          <div className="loyalty-mgmt-header">
            <Skeleton variant="block" width="6rem" height="1.75rem" />
            <div className="loyalty-mgmt-tabs">
              <Skeleton variant="block" width="5rem" height="2.25rem" />
              <Skeleton variant="block" width="4rem" height="2.25rem" />
            </div>
          </div>
          <div className="loyalty-table-wrap">
            <table className="loyalty-table" aria-hidden="true">
              <thead>
                <tr>
                  {['Customer', 'Tier', 'Points', 'Lifetime Points', 'Next Tier', 'Points to Next', ''].map((_, i) => (
                    <th key={i}><Skeleton variant="text" width={i < 6 ? '5rem' : '3rem'} height="0.75rem" /></th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {[0, 1, 2, 3].map((r) => (
                  <tr key={r}>
                    <td><Skeleton variant="text" width="6rem" height="0.875rem" /></td>
                    <td><Skeleton variant="block" width="4rem" height="1.125rem" style={{ borderRadius: 'var(--radius-full)' }} /></td>
                    <td><Skeleton variant="text" width="3rem" height="0.75rem" /></td>
                    <td><Skeleton variant="text" width="4rem" height="0.75rem" /></td>
                    <td><Skeleton variant="text" width="4rem" height="0.75rem" /></td>
                    <td><Skeleton variant="text" width="3rem" height="0.75rem" /></td>
                    <td><Skeleton variant="circle" width="1.25rem" height="1.25rem" /></td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      ) : tierTab ? (
        <div className="loyalty-tiers-section">
          <div className="loyalty-tiers-grid">
            {tiers.map((tier) => (
              <Card key={tier.id} shadow="sm" className="loyalty-tier-card">
                {editingTier === tier.id ? (
                  <div className="loyalty-tier-edit-form">
                    <div className="loyalty-tier-field">
                      <Localized id="loyalty-tier-name"><span className="loyalty-tier-label">Name</span></Localized>
                      <Localized id="loyalty-tier-name-aria" attrs={{ 'aria-label': true }}>
                      {/* eslint-disable-next-line jsx-a11y/control-has-associated-label -- aria-label set via Localized above */}
                      <input className="loyalty-tier-input" value={tierForm.name} onChange={(e) => setTierForm({ ...tierForm, name: e.target.value })} />
                      </Localized>
                    </div>
                    <div className="loyalty-tier-field">
                      <Localized id="loyalty-tier-min-points"><span className="loyalty-tier-label">Min Points</span></Localized>
                      <Localized id="loyalty-tier-min-points-aria" attrs={{ 'aria-label': true }}>
                      {/* eslint-disable-next-line jsx-a11y/control-has-associated-label -- aria-label set via Localized above */}
                      <input className="loyalty-tier-input" type="number" value={tierForm.min_points} onChange={(e) => setTierForm({ ...tierForm, min_points: e.target.value })} />
                      </Localized>
                    </div>
                    <div className="loyalty-tier-field">
                      <Localized id="loyalty-tier-ppu"><span className="loyalty-tier-label">Points/Unit</span></Localized>
                      <Localized id="loyalty-tier-ppu-aria" attrs={{ 'aria-label': true }}>
                      {/* eslint-disable-next-line jsx-a11y/control-has-associated-label -- aria-label set via Localized above */}
                      <input className="loyalty-tier-input" type="number" value={tierForm.points_per_unit} onChange={(e) => setTierForm({ ...tierForm, points_per_unit: e.target.value })} />
                      </Localized>
                    </div>
                    <div className="loyalty-tier-field">
                      <Localized id="loyalty-tier-multiplier"><span className="loyalty-tier-label">Multiplier</span></Localized>
                      <Localized id="loyalty-tier-multiplier-aria" attrs={{ 'aria-label': true }}>
                      {/* eslint-disable-next-line jsx-a11y/control-has-associated-label -- aria-label set via Localized above */}
                      <input className="loyalty-tier-input" type="number" step="0.01" value={tierForm.earn_multiplier} onChange={(e) => setTierForm({ ...tierForm, earn_multiplier: e.target.value })} />
                      </Localized>
                    </div>
                    <div className="loyalty-tier-field">
                      <Localized id="loyalty-tier-colour"><span className="loyalty-tier-label">Colour</span></Localized>
                      <Localized id="loyalty-tier-colour-aria" attrs={{ 'aria-label': true }}>
                      {/* eslint-disable-next-line jsx-a11y/control-has-associated-label -- aria-label set via Localized above */}
                      <input className="loyalty-tier-input loyalty-tier-colour-input" type="color" value={tierForm.colour} onChange={(e) => setTierForm({ ...tierForm, colour: e.target.value })} />
                      </Localized>
                    </div>
                    {error && <div className="loyalty-mgmt-error" role="alert">{error}</div>}
                    <div className="loyalty-tier-edit-actions">
                      <Button variant="ghost" onClick={() => setEditingTier(null)} disabled={savingTier}>
                        <Localized id="cancel">Cancel</Localized>
                      </Button>
                      <Button variant="primary" loading={savingTier} onClick={handleSaveTier}>
                        <Localized id="save">Save</Localized>
                      </Button>
                    </div>
                  </div>
                ) : (
                  <>
                    <div className="loyalty-tier-head" style={{ borderLeftColor: tier.colour }}>
                      <h3 className="loyalty-tier-name">{tier.name}</h3>
                      <span className="loyalty-tier-badge" style={{ background: tier.colour, color: '#fff' }}>{tier.name}</span>
                    </div>
                    <div className="loyalty-tier-details">
                      <div className="loyalty-tier-detail">
                        <Localized id="loyalty-tier-min-points"><span>Min Points</span></Localized>
                        <span>{tier.min_points.toLocaleString()}</span>
                      </div>
                      <div className="loyalty-tier-detail">
                        <Localized id="loyalty-tier-ppu"><span>Points/Unit</span></Localized>
                        <span>{tier.points_per_unit}</span>
                      </div>
                      <div className="loyalty-tier-detail">
                        <Localized id="loyalty-tier-multiplier"><span>Multiplier</span></Localized>
                        <span>{tier.earn_multiplier}x</span>
                      </div>
                    </div>
                    <Localized id="edit">
                      {/* eslint-disable-next-line jsx-a11y/control-has-associated-label -- button has visible text 'Edit' */}
                      <button type="button" className="loyalty-tier-edit-btn" onClick={() => openEditTier(tier)}>Edit</button>
                    </Localized>
                  </>
                )}
              </Card>
            ))}
          </div>
        </div>
      ) : (
        <div className="loyalty-accounts-section">
          {accounts.length === 0 ? (
            <Card shadow="sm">
              <div className="loyalty-mgmt-empty">
                <Localized id="loyalty-no-accounts">
                  <p>No loyalty accounts yet</p>
                </Localized>
              </div>
            </Card>
          ) : (
            <div className="loyalty-table-wrap">
              <table className="loyalty-table" aria-label={l10n.getString('loyalty-table-aria')}>
                <thead>
                  <tr>
                    <Localized id="loyalty-customer"><th>Customer</th></Localized>
                    <Localized id="loyalty-tier"><th>Tier</th></Localized>
                    <Localized id="loyalty-points"><th>Points</th></Localized>
                    <Localized id="loyalty-lifetime-points"><th>Lifetime Points</th></Localized>
                    <Localized id="loyalty-next-tier"><th>Next Tier</th></Localized>
                    <Localized id="loyalty-points-to-next"><th>Points to Next</th></Localized>
                    <Localized id="loyalty-table-actions" attrs={{ 'aria-label': true }}>
                      {/* eslint-disable-next-line jsx-a11y/control-has-associated-label -- aria-label set via Localized attrs */}
                      <th> </th>
                    </Localized>
                  </tr>
                </thead>
                <tbody>
                  {accounts.map((a) => {
                    const customerName = customerMap.get(a.account.customer_id) ?? a.account.customer_id;
                    const isExpanded = selectedAccount === a.account.id;
                    return (
                      <Fragment key={a.account.id}>
                        <tr className="loyalty-table-row" tabIndex={0} role="button" aria-expanded={isExpanded} aria-label={l10n.getString(isExpanded ? 'loyalty-collapse' : 'loyalty-expand')} onClick={() => setSelectedAccount(isExpanded ? null : a.account.id)} onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); setSelectedAccount(isExpanded ? null : a.account.id); } }}>
                          <td><span className="loyalty-customer-name">{customerName}</span></td>
                          <td>
                            {a.tier ? (
                              <span className="loyalty-tier-badge" style={{ background: a.tier.colour, color: '#fff' }}>
                                {a.tier.name}
                              </span>
                            ) : (
                              <span className="loyalty-tier-badge loyalty-tier-badge--none">—</span>
                            )}
                          </td>
                          <td className="loyalty-points-cell">{a.account.points.toLocaleString()}</td>
                          <td className="loyalty-points-cell">{a.account.lifetime_points.toLocaleString()}</td>
                          <td>{a.next_tier?.name ?? '—'}</td>
                          <td>{a.points_to_next_tier > 0 ? a.points_to_next_tier.toLocaleString() : '—'}</td>
                          <td>
                            {/* eslint-disable-next-line jsx-a11y/control-has-associated-label -- aria-label set dynamically */}
                            <button type="button" className="loyalty-expand-btn" aria-label={l10n.getString(isExpanded ? 'loyalty-collapse' : 'loyalty-expand')} aria-expanded={isExpanded}>
                              {isExpanded ? '\u25B2' : '\u25BC'}
                            </button>
                          </td>
                        </tr>
                        {isExpanded && (
                          <tr className="loyalty-detail-row">
                            <td colSpan={7}>
                              <div className="loyalty-detail-content">
                                <Localized id="loyalty-recent-transactions">
                                  <h4 className="loyalty-detail-title">Recent Activity</h4>
                                </Localized>
                                {a.recent_transactions.length === 0 ? (
                                  <p className="loyalty-detail-empty"><Localized id="loyalty-no-transactions">No transactions yet</Localized></p>
                                ) : (
                                  <table className="loyalty-txn-table" aria-label={l10n.getString('loyalty-txn-table-aria')}>
                                    <thead>
                                      <tr>
                                        <Localized id="loyalty-txn-type"><th>Type</th></Localized>
                                        <Localized id="loyalty-txn-points"><th>Points</th></Localized>
                                        <Localized id="loyalty-txn-description"><th>Description</th></Localized>
                                        <Localized id="loyalty-txn-date"><th>Date</th></Localized>
                                      </tr>
                                    </thead>
                                    <tbody>
                                      {a.recent_transactions.map((txn) => (
                                        <tr key={txn.id}>
                                          {/* eslint-disable-next-line jsx-a11y/control-has-associated-label -- aria-label set dynamically */}
                                          <td>
                                            <span className={`loyalty-txn-type loyalty-txn-type--${txn.txn_type}`}>
                                              <Localized id={`loyalty-${txn.txn_type}`}>
                                                <span>{txn.txn_type.charAt(0).toUpperCase() + txn.txn_type.slice(1)}</span>
                                              </Localized>
                                            </span>
                                          </td>
                                          <td className={`loyalty-points-cell ${txn.points < 0 ? 'loyalty-points-negative' : 'loyalty-points-positive'}`}>
                                            {txn.points > 0 ? `+${txn.points}` : txn.points}
                                          </td>
                                          <td>{txn.description}</td>
                                          <td className="loyalty-txn-date">{new Date(txn.created_at).toLocaleDateString()}</td>
                                        </tr>
                                      ))}
                                    </tbody>
                                    </table>
                                )}
                              </div>
                            </td>
                          </tr>
                        )}
                      </Fragment>
                    );
                  })}
                </tbody>
                </table>
              </div>
          )}
        </div>
      )}
    </div>
  );
}
