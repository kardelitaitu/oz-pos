import { useState, useEffect, useCallback } from 'react';
import { useLocalization } from '@fluent/react';
import { readScaleWeight, type WeightReading } from '@/api/hardware';
import type { Sku } from '@/types/domain';

interface ScaleIndicatorProps {
  weighTarget: { sku: Sku; name: string } | null;
  onWeighAdd: (sku: Sku, weightGrams: number) => void;
  onClearWeighTarget: () => void;
}

export default function ScaleIndicator({ weighTarget, onWeighAdd, onClearWeighTarget }: ScaleIndicatorProps) {
  const { l10n } = useLocalization();
  const [reading, setReading] = useState<WeightReading | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    const poll = async () => {
      try {
        const r = await readScaleWeight();
        if (!cancelled) {
          setReading(r);
          setError(null);
        }
      } catch {
        if (!cancelled) setError(l10n.getString('scale-read-error') || 'Scale error');
      }
    };
    poll();
    const id = setInterval(poll, 2000);
    return () => { cancelled = true; clearInterval(id); };
  }, [l10n]);

  const formatWeight = (g: number): string => {
    if (g >= 1000) return `${(g / 1000).toFixed(2)} kg`;
    return `${g.toFixed(0)} g`;
  };

  const handleWeighAdd = useCallback(() => {
    if (weighTarget && reading?.stable && reading.weightGrams > 0) {
      onWeighAdd(weighTarget.sku, reading.weightGrams);
    }
  }, [weighTarget, reading, onWeighAdd]);

  if (error) {
    return (
      <div className="scale-indicator scale-indicator--error" role="status" aria-label={l10n.getString('scale-indicator-aria') || 'Scale'}>
        <span className="scale-indicator-error">{error}</span>
      </div>
    );
  }

  if (!reading) {
    return (
      <div className="scale-indicator scale-indicator--idle" role="status" aria-label={l10n.getString('scale-indicator-aria') || 'Scale'}>
        <span className="scale-indicator-label">{l10n.getString('scale-idle') || 'Scale'}</span>
      </div>
    );
  }

  return (
    <div className={`scale-indicator ${reading.stable ? 'scale-indicator--stable' : 'scale-indicator--unstable'}`} role="status" aria-label={l10n.getString('scale-indicator-aria') || 'Scale weight indicator'}>
      <div className="scale-indicator-display">
        <span className={`scale-indicator-dot ${reading.stable ? 'scale-indicator-dot--stable' : 'scale-indicator-dot--unstable'}`} aria-hidden="true" />
        <span className="scale-indicator-weight">{formatWeight(reading.weightGrams)}</span>
        <span className="scale-indicator-stable-label">
          {reading.stable ? (l10n.getString('scale-stable') || 'Stable') : (l10n.getString('scale-unstable') || '…')}
        </span>
      </div>
      {weighTarget && reading.stable && reading.weightGrams > 0 && (
        <div className="scale-indicator-actions">
          <span className="scale-indicator-target">
            <svg viewBox="0 0 20 20" fill="currentColor" width="12" height="12" aria-hidden="true">
              <path d="M10 10a4 4 0 100-8 4 4 0 000 8zm-7 8a7 7 0 1114 0H3z" />
            </svg>
            {weighTarget.name}
          </span>
          <button
            type="button"
            className="scale-indicator-add-btn"
            onClick={handleWeighAdd}
            aria-label={l10n.getString('scale-weigh-add-aria', { name: weighTarget.name }) || `Weigh & add ${weighTarget.name}`}
          >
            {l10n.getString('scale-weigh-add') || 'Weigh & Add'}
          </button>
          <button
            type="button"
            className="scale-indicator-clear-btn"
            onClick={onClearWeighTarget}
            aria-label={l10n.getString('scale-clear-aria') || 'Clear'}
          >
            &times;
          </button>
        </div>
      )}
    </div>
  );
}
