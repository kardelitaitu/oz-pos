import { useState, useCallback } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import { Button } from '@/components/Button';
import { useWorkspace } from '@/contexts/WorkspaceContext';
import { loggedInvoke } from '@/utils/logged-invoke';
import './ExitSurveyModal.css';

/**
 * Exit survey reasons (§7 churn prevention).
 * Shown when a user pauses their subscription to understand churn drivers.
 */
const SURVEY_REASONS = [
  { value: 'too_expensive', labelKey: 'exit-survey-reason-price' },
  { value: 'not_enough_features', labelKey: 'exit-survey-reason-features' },
  { value: 'switching_competitor', labelKey: 'exit-survey-reason-competitor' },
  { value: 'business_closed', labelKey: 'exit-survey-reason-closed' },
  { value: 'temporary_break', labelKey: 'exit-survey-reason-break' },
  { value: 'other', labelKey: 'exit-survey-reason-other' },
] as const;

interface ExitSurveyModalProps {
  open: boolean;
  onClose: () => void;
  onConfirm: () => void;
}

/**
 * §7: Exit survey modal — collects churn reasons when a user pauses
 * their subscription. The feedback is stored server-side for analysis
 * and helps prioritize retention improvements.
 */
export default function ExitSurveyModal({ open, onClose, onConfirm }: ExitSurveyModalProps) {
  const { l10n } = useLocalization();
  const { sessionToken: rawToken } = useWorkspace();
  const sessionToken = rawToken || '';
  const [selectedReason, setSelectedReason] = useState<string>('');
  const [otherText, setOtherText] = useState('');
  const [submitting, setSubmitting] = useState(false);

  const handleSubmit = useCallback(async () => {
    if (!selectedReason) return;

    setSubmitting(true);
    try {
      // Best-effort: store the feedback reason via the settings API.
      // If this fails, we still proceed with the pause.
      try {
        await loggedInvoke('set_exit_survey_response', {
          sessionToken,
          reason: selectedReason,
          detail: selectedReason === 'other' ? otherText : undefined,
        });
      } catch {
        // Non-critical — log and continue
        console.warn('Failed to save exit survey response');
      }
      onConfirm();
    } finally {
      setSubmitting(false);
    }
  }, [selectedReason, otherText, sessionToken, onConfirm]);

  if (!open) return null;

  return (
    <div className="exit-survey-overlay" role="dialog" aria-label={l10n.getString('exit-survey-title')}>
      <div className="exit-survey-modal">
        <h2 className="exit-survey-title">
          <Localized id="exit-survey-title">
            <span>Before you pause...</span>
          </Localized>
        </h2>
        <p className="exit-survey-message">
          <Localized id="exit-survey-message">
            <span>Help us improve — what&apos;s the main reason you&apos;re pausing?</span>
          </Localized>
        </p>

        <div className="exit-survey-reasons" role="radiogroup" aria-label={l10n.getString('exit-survey-title')}>
          {SURVEY_REASONS.map(({ value, labelKey }) => (
            <span key={value} className="exit-survey-reason">
              <input
                type="radio"
                id={`exit-reason-${value}`}
                name="exit-survey-reason"
                value={value}
                checked={selectedReason === value}
                onChange={() => setSelectedReason(value)}
                className="exit-survey-radio"
                aria-labelledby={`exit-label-${value}`}
              />
              <Localized id={labelKey}>
                <span id={`exit-label-${value}`}>{value}</span>
              </Localized>
            </span>
          ))}
        </div>

        {selectedReason === 'other' && (
          <textarea
            className="exit-survey-other"
            placeholder={l10n.getString('exit-survey-other-placeholder')}
            value={otherText}
            onChange={(e) => setOtherText(e.target.value)}
            rows={3}
            aria-label={l10n.getString('exit-survey-other-placeholder')}
          />
        )}

        <div className="exit-survey-actions">
          <Button variant="secondary" onClick={onClose} disabled={submitting}>
            <Localized id="exit-survey-cancel">
              <span>Go back</span>
            </Localized>
          </Button>
          <Button
            variant="primary"
            onClick={handleSubmit}
            disabled={!selectedReason || submitting}
            loading={submitting}
          >
            <Localized id="exit-survey-submit">
              <span>Pause subscription</span>
            </Localized>
          </Button>
        </div>
      </div>
    </div>
  );
}
