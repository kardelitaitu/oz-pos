import { useState, useCallback } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import { createStockCount, type StockCountDto } from '@/api/inventoryCounts';
import { Card } from '@/components/Card';
import { Button } from '@/components/Button';
import './StockCountForm.css';

interface Props {
  onCreated: (count: StockCountDto) => void;
  onCancel: () => void;
}

/** New stock count creation form — select count type (full, cyclic, or spot) and add optional notes before creating. */
export default function StockCountForm({ onCreated, onCancel }: Props) {
  const [countType, setCountType] = useState<'full' | 'cyclic' | 'spot'>('full');
  const [notes, setNotes] = useState('');
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const { l10n } = useLocalization();

  const handleSubmit = useCallback(async () => {
    setSaving(true);
    setError(null);
    try {
      const result = await createStockCount({
        countType,
        notes,
      });
      onCreated(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : l10n.getString('sc-error-create'));
    } finally {
      setSaving(false);
    }
  }, [countType, notes, onCreated, l10n]);

  const typeOptions: Array<{ value: 'full' | 'cyclic' | 'spot' }> = [
    { value: 'full' },
    { value: 'cyclic' },
    { value: 'spot' },
  ];

  return (
    <div className="sc-form-screen">
      <div className="sc-form-header">
        <h1 className="sc-title">
          <Localized id="sc-new-count-title">
            <span>New Stock Count</span>
          </Localized>
        </h1>
      </div>

      <Card shadow="sm" className="sc-form-card">
        <div className="sc-form-field" role="radiogroup" aria-label={l10n.getString('sc-type-aria')}>
          <div className="sc-form-label">
            <Localized id="sc-type-label">
              <span>Count Type</span>
            </Localized>
          </div>
          <div className="sc-type-options">
            {typeOptions.map((opt) => (
              <button
                key={opt.value}
                type="button"
                className={`sc-type-btn ${countType === opt.value ? 'sc-type-btn--active' : ''}`}
                onClick={() => setCountType(opt.value)}
                role="radio"
                aria-checked={countType === opt.value}
              >
                <Localized id={`sc-type-${opt.value}`}>
                  <span>{opt.value}</span>
                </Localized>
              </button>
            ))}
          </div>
        </div>

        <div className="sc-form-field">
          <div className="sc-form-label">
            <Localized id="sc-notes-label">
              <span>Notes (optional)</span>
            </Localized>
          </div>            {/* eslint-disable-next-line jsx-a11y/control-has-associated-label -- aria-label provided */}
            <textarea
            id="sc-notes"
            className="sc-form-textarea"
            value={notes}
            onChange={(e) => setNotes(e.target.value)}
            placeholder={l10n.getString('sc-notes-placeholder')}
            rows={3}
            aria-label={l10n.getString('sc-notes-label')}
          />
        </div>

        {error && (
          <div className="sc-form-error" role="alert">
            {error}
          </div>
        )}

        <div className="sc-form-actions">
          <Button variant="ghost" onClick={onCancel}>
            <Localized id="sc-cancel">
              <span>Cancel</span>
            </Localized>
          </Button>
          <Button variant="primary" onClick={handleSubmit} loading={saving}>
            <Localized id="sc-start-count">
              <span>Start Count</span>
            </Localized>
          </Button>
        </div>
      </Card>
    </div>
  );
}
