import { useState, useCallback, useEffect } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import {
  listGiftCards,
  freezeGiftCard,
  unfreezeGiftCard,
  topUpGiftCard,
  type GiftCardWithTransactions,
  type GiftCardFilter,
} from '@/api/giftCards';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import IssueGiftCardModal from './IssueGiftCardModal';
import './GiftCardsScreen.css';

const STATUS_CLASS: Record<string, string> = {
  active: 'gift-card-status--active',
  frozen: 'gift-card-status--frozen',
  redeemed: 'gift-card-status--redeemed',
  expired: 'gift-card-status--expired',
};

export default function GiftCardsScreen() {
  const { l10n } = useLocalization();
  const [cards, setCards] = useState<GiftCardWithTransactions[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState('');
  const [statusFilter, setStatusFilter] = useState('');
  const [showIssueModal, setShowIssueModal] = useState(false);
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [topUpCardId, setTopUpCardId] = useState<string | null>(null);
  const [topUpAmount, setTopUpAmount] = useState('');
  const [topUpError, setTopUpError] = useState('');

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const filter: GiftCardFilter = {};
      if (search.trim()) filter.search = search.trim();
      if (statusFilter) filter.status = statusFilter;
      const result = await listGiftCards(filter);
      setCards(result);
    } catch {
      // IPC unavailable.
    } finally {
      setLoading(false);
    }
  }, [search, statusFilter]);

  useEffect(() => { load(); }, [load]);

  const handleFreezeToggle = useCallback(async (cardNumber: string, currentStatus: string) => {
    try {
      if (currentStatus === 'frozen') {
        await unfreezeGiftCard(cardNumber);
      } else {
        await freezeGiftCard(cardNumber);
      }
      await load();
    } catch (err) {
      console.error('Failed to toggle freeze:', err);
    }
  }, [load]);

  const handleTopUp = useCallback(async (cardNumber: string) => {
    const amount = parseInt(topUpAmount, 10);
    if (Number.isNaN(amount) || amount <= 0) {
      setTopUpError(l10n.getString('gift-cards-topup-invalid'));
      return;
    }
    setTopUpError('');
    try {
      await topUpGiftCard(cardNumber, amount);
      setTopUpCardId(null);
      setTopUpAmount('');
      await load();
    } catch (err) {
      setTopUpError(err instanceof Error ? err.message : 'Top-up failed');
    }
  }, [topUpAmount, load, l10n]);

  const formatMoney = (minor: number, currency: string): string => {
    const known: Record<string, number> = { JPY: 0, KRW: 0, VND: 0, IDR: 2 };
    const exp = known[currency] ?? 2;
    const val = (minor / 10 ** exp).toLocaleString(undefined, {
      minimumFractionDigits: exp,
      maximumFractionDigits: exp,
    });
    return `${currency} ${val}`;
  };

  return (
    <div className="gift-cards-page">
      <div className="gift-cards-header">
        <Localized id="gift-cards-title">
          <h1 className="gift-cards-title">Gift Cards</h1>
        </Localized>
        <Button variant="primary" onClick={() => setShowIssueModal(true)}>
          <Localized id="gift-cards-issue-btn">+ Issue New Card</Localized>
        </Button>
      </div>

      <div className="gift-cards-toolbar">
        <input
          type="text"
          className="gift-cards-search"
          placeholder={l10n.getString('gift-cards-search-placeholder')}
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          aria-label={l10n.getString('gift-cards-search-aria')}
        />
        <select
          className="gift-cards-status-filter"
          value={statusFilter}
          onChange={(e) => setStatusFilter(e.target.value)}
          aria-label={l10n.getString('gift-cards-status-aria')}
        >
          <Localized id="gift-cards-status-all"><option value="">All Statuses</option></Localized>
          <Localized id="gift-cards-status-active"><option value="active">Active</option></Localized>
          <Localized id="gift-cards-status-frozen"><option value="frozen">Frozen</option></Localized>
          <Localized id="gift-cards-status-redeemed"><option value="redeemed">Redeemed</option></Localized>
          <Localized id="gift-cards-status-expired"><option value="expired">Expired</option></Localized>
        </select>
      </div>

      {loading ? (
        <Localized id="gift-cards-loading"><p className="gift-cards-loading">Loading...</p></Localized>
      ) : cards.length === 0 ? (
        <Card shadow="sm">
          <div className="gift-cards-empty">
            <Localized id="gift-cards-no-cards">
              <p>No gift cards found</p>
            </Localized>
          </div>
        </Card>
      ) : (
        <div className="gift-cards-list">
          {cards.map((gc) => (
            <Card key={gc.card.id} shadow="sm" className="gift-card-card">
              <div
                className="gift-card-summary"
                role="button"
                tabIndex={0}
                onClick={() => setExpandedId(expandedId === gc.card.id ? null : gc.card.id)}
                onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); setExpandedId(expandedId === gc.card.id ? null : gc.card.id); } }}
              >
                <div className="gift-card-summary-left">
                  <span className="gift-card-number">{gc.card.card_number}</span>
                  {gc.card.issued_to && (
                    <span className="gift-card-issued-to">{gc.card.issued_to}</span>
                  )}
                </div>
                <div className="gift-card-summary-right">
                  <span className={`gift-card-status ${STATUS_CLASS[gc.card.status] || ''}`}>
                    {gc.card.status}
                  </span>
                  <span className="gift-card-balance">
                    {formatMoney(gc.card.current_balance_minor, gc.card.currency)}
                  </span>
                  <span className={`gift-card-expand ${expandedId === gc.card.id ? 'expanded' : ''}`}>
                    &#9660;
                  </span>
                </div>
              </div>

              {expandedId === gc.card.id && (
                <div className="gift-card-detail">
                  <div className="gift-card-info-grid">
                    <div className="gift-card-info-item">
                      <Localized id="gift-cards-info-initial-balance"><span className="gift-card-info-label">Initial Balance</span></Localized>
                      <span>{formatMoney(gc.card.initial_balance_minor, gc.card.currency)}</span>
                    </div>
                    <div className="gift-card-info-item">
                      <Localized id="gift-cards-info-issued"><span className="gift-card-info-label">Issued</span></Localized>
                      <span>{new Date(gc.card.issue_date).toLocaleDateString()}</span>
                    </div>
                    {gc.card.expiry_date && (
                      <div className="gift-card-info-item">
                        <Localized id="gift-cards-info-expires"><span className="gift-card-info-label">Expires</span></Localized>
                        <span>{new Date(gc.card.expiry_date).toLocaleDateString()}</span>
                      </div>
                    )}
                  </div>

                  <div className="gift-card-actions">
                    {gc.card.status === 'active' || gc.card.status === 'frozen' ? (
                      <Button
                        variant="ghost"
                        onClick={() => handleFreezeToggle(gc.card.card_number, gc.card.status)}
                      >
                        {gc.card.status === 'frozen' ? (
                          <Localized id="gift-cards-unfreeze"><span>Unfreeze</span></Localized>
                        ) : (
                          <Localized id="gift-cards-freeze"><span>Freeze</span></Localized>
                        )}
                      </Button>
                    ) : null}
                    {gc.card.status === 'active' && (
                      <Button variant="primary" onClick={() => setTopUpCardId(gc.card.id)}>
                        <Localized id="gift-cards-top-up"><span>Top Up</span></Localized>
                      </Button>
                    )}
                  </div>

                  {topUpCardId === gc.card.id && (
                    <div className="gift-card-topup-form">
                      <input
                        type="number"
                        className="gift-card-topup-input"
                        placeholder="Amount (minor units)"
                        value={topUpAmount}
                        onChange={(e) => { setTopUpAmount(e.target.value); setTopUpError(''); }}
                        aria-label="Top-up amount"
                      />
                      <Localized id="gift-cards-confirm-topup">
                        <Button variant="primary" onClick={() => handleTopUp(gc.card.card_number)}>
                          <span>Confirm Top-Up</span>
                        </Button>
                      </Localized>
                      <Localized id="gift-cards-cancel-topup">
                        <Button variant="ghost" onClick={() => { setTopUpCardId(null); setTopUpAmount(''); }}>
                          <span>Cancel</span>
                        </Button>
                      </Localized>
                      {topUpError && <div className="gift-card-topup-error">{topUpError}</div>}
                    </div>
                  )}

                  {gc.transactions.length > 0 && (
                    <div className="gift-card-transactions">
                      <Localized id="gift-cards-recent-transactions">
                        <h4 className="gift-card-txn-title">Recent Transactions</h4>
                      </Localized>
                      <table className="gift-card-txn-table">
                        <thead>
                          <tr>
                            <Localized id="gift-cards-txn-type"><th>Type</th></Localized>
                            <Localized id="gift-cards-txn-amount"><th>Amount</th></Localized>
                            <Localized id="gift-cards-txn-balance"><th>Balance</th></Localized>
                            <Localized id="gift-cards-txn-notes"><th>Notes</th></Localized>
                            <Localized id="gift-cards-txn-date"><th>Date</th></Localized>
                          </tr>
                        </thead>
                        <tbody>
                          {gc.transactions.map((txn) => (
                            <tr key={txn.id}>
                              <td>
                                <span className={`gift-card-txn-type gift-card-txn-type--${txn.txn_type}`}>
                                  {txn.txn_type}
                                </span>
                              </td>
                              <td className={`gift-card-txn-amount ${txn.amount_minor < 0 ? 'negative' : 'positive'}`}>
                                {txn.amount_minor > 0 ? `+${txn.amount_minor}` : txn.amount_minor}
                              </td>
                              <td>{txn.balance_after_minor}</td>
                              <td className="gift-card-txn-notes">{txn.notes}</td>
                              <td className="gift-card-txn-date">{new Date(txn.created_at).toLocaleDateString()}</td>
                            </tr>
                          ))}
                        </tbody>
                      </table>
                    </div>
                  )}
                </div>
              )}
            </Card>
          ))}
        </div>
      )}

      {showIssueModal && (
        <IssueGiftCardModal
          onClose={() => setShowIssueModal(false)}
          onIssued={() => {
            setShowIssueModal(false);
            load();
          }}
        />
      )}
    </div>
  );
}
