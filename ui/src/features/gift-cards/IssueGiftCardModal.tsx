import { useState } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import { issueGiftCard, type IssueGiftCardInput } from '@/api/giftCards';
import { Button } from '@/components/Button';
import { generateGiftCardNumber } from '@/utils/giftCardBarcode';

export interface IssueGiftCardModalProps {
  onClose: () => void;
  onIssued: () => void;
}

export default function IssueGiftCardModal({ onClose, onIssued }: IssueGiftCardModalProps) {
  const { l10n } = useLocalization();
  const [cardNumber, setCardNumber] = useState(generateGiftCardNumber());
  const [amount, setAmount] = useState('');
  const [issuedTo, setIssuedTo] = useState('');
  const [pin, setPin] = useState('');
  const [processing, setProcessing] = useState(false);
  const [error, setError] = useState('');

  const handleSubmit = async () => {
    const initialAmountMinor = parseInt(amount, 10);
    if (Number.isNaN(initialAmountMinor) || initialAmountMinor <= 0) {
      setError(l10n.getString('gift-cards-issue-invalid-amount'));
      return;
    }
    if (!cardNumber.trim()) {
      setError(l10n.getString('gift-cards-issue-invalid-number'));
      return;
    }

    setProcessing(true);
    setError('');

    try {
      const input: IssueGiftCardInput = {
        card_number: cardNumber.trim(),
        initial_amount_minor: initialAmountMinor,
        currency: 'IDR',
        issued_to: issuedTo.trim() || null,
        created_by: 'staff',
        pin: pin.trim() || null,
      };
      await issueGiftCard(input);
      onIssued();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to issue gift card');
    } finally {
      setProcessing(false);
    }
  };

  return (
    <div className="gift-cards-modal-overlay" role="button" tabIndex={0} aria-label="Close" onClick={onClose} onKeyDown={(e) => { if (e.key === 'Escape' || e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onClose(); } }}>
      <div className="gift-cards-modal" role="presentation" onClick={(e) => e.stopPropagation()}>
        <Localized id="gift-cards-issue-title">
          <h2 className="gift-cards-modal-title">Issue Gift Card</h2>
        </Localized>

        <div className="gift-cards-modal-form">
          <div className="gift-cards-modal-field">
            <Localized id="gift-cards-issue-number-label">
              <div className="gift-cards-modal-label">Card Number</div>
            </Localized>
            <input
              type="text"
              className="gift-cards-modal-input"
              value={cardNumber}
              onChange={(e) => { setCardNumber(e.target.value); setError(''); }}
              aria-label={l10n.getString('gift-cards-issue-number-aria')}
            />
          </div>

          <div className="gift-cards-modal-field">
            <Localized id="gift-cards-issue-amount-label">
              <div className="gift-cards-modal-label">Initial Amount (minor units)</div>
            </Localized>
            <input
              type="number"
              className="gift-cards-modal-input"
              placeholder="e.g. 50000"
              value={amount}
              onChange={(e) => { setAmount(e.target.value); setError(''); }}
              aria-label={l10n.getString('gift-cards-issue-amount-aria')}
            />
          </div>

          <div className="gift-cards-modal-field">
            <Localized id="gift-cards-issue-to-label">
              <div className="gift-cards-modal-label">Issued To (optional)</div>
            </Localized>
            <input
              type="text"
              className="gift-cards-modal-input"
              placeholder="Customer name"
              value={issuedTo}
              onChange={(e) => setIssuedTo(e.target.value)}
              aria-label={l10n.getString('gift-cards-issue-to-aria')}
            />
          </div>

          <div className="gift-cards-modal-field">
            <Localized id="gift-cards-issue-pin-label">
              <div className="gift-cards-modal-label">PIN (optional)</div>
            </Localized>
            <input
              type="text"
              className="gift-cards-modal-input"
              placeholder="For balance checks"
              value={pin}
              onChange={(e) => setPin(e.target.value)}
              aria-label={l10n.getString('gift-cards-issue-pin-aria')}
            />
          </div>

          {error && <div className="gift-cards-modal-error" role="alert">{error}</div>}

          <div className="gift-cards-modal-actions">
            <Button variant="ghost" onClick={onClose} disabled={processing}>
              <Localized id="cancel">Cancel</Localized>
            </Button>
            <Button variant="primary" loading={processing} onClick={handleSubmit}>
              <Localized id="gift-cards-issue-confirm">Issue Card</Localized>
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
