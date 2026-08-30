import { useRef, useState } from 'react';
import { t } from '../../i18n';
import { type Region } from '../../lib/region';

/** Region options for the billing-region selector. */
export const REGION_OPTIONS: { value: Region; labelKey: string }[] = [
  { value: 'global', labelKey: 'signup.regionGlobal' },
  { value: 'id', labelKey: 'signup.regionIndonesia' },
];

interface Props {
  locale: string;
  region: Region;
  onRegionChange: (region: Region) => void;
}

/**
 * Billing-region selector — a custom listbox with full keyboard support
 * (Arrow keys move focus, Escape closes, Enter/Space select) and a blur
 * guard that keeps the listbox open while the user navigates its options.
 * Owns only its open/confirm-feedback state; the chosen region is lifted to
 * the parent (it drives payment routing).
 */
export default function AccountRegion({ locale, region, onRegionChange }: Props) {
  const [regionOpen, setRegionOpen] = useState(false);
  const [regionMsg, setRegionMsg] = useState(false);
  const timerRef = useRef<number | null>(null);

  const closeSoon = () => {
    if (timerRef.current !== null) window.clearTimeout(timerRef.current);
    timerRef.current = window.setTimeout(() => setRegionOpen(false), 150);
  };

  return (
    <section className="rounded-xl border border-ink/10 bg-surface/40 p-6 shadow-sm" aria-label={t(locale, 'account.region')}>
      <h2 className="text-lg font-semibold">{t(locale, 'account.region')}</h2>
      <p className="mt-1 text-sm text-muted">{t(locale, 'account.regionHint')}</p>
      <div className="relative mt-3">
        <button
          type="button"
          onClick={() => setRegionOpen(!regionOpen)}
          onBlur={(e) => {
            // Only close when focus leaves the whole listbox. When the
            // user keyboard-navigates to an option, focus moves to a
            // button inside the listbox — that blur must NOT close it,
            // otherwise a keyboard user loses the dropdown mid-arrow.
            if (e.relatedTarget instanceof HTMLElement && e.relatedTarget.closest('[role="listbox"]')) {
              return;
            }
            closeSoon();
          }}
          onKeyDown={(e) => {
            // ArrowDown/ArrowUp open the listbox and move focus to the first option;
            // Escape closes it.
            if (!regionOpen && (e.key === 'ArrowDown' || e.key === 'ArrowUp')) {
              e.preventDefault();
              setRegionOpen(true);
              window.setTimeout(() => {
                const first = document.querySelector<HTMLButtonElement>('[data-region-option]');
                first?.focus();
              }, 0);
            } else if (regionOpen && e.key === 'Escape') {
              setRegionOpen(false);
              e.currentTarget.focus();
            }
          }}
          aria-haspopup="listbox"
          aria-expanded={regionOpen}
          className="w-full rounded-md border border-ink/10 bg-surface px-3 py-2 text-sm text-left outline-none transition focus:border-accent flex items-center justify-between"
        >
          <span>{t(locale, region === 'id' ? 'signup.regionIndonesia' : 'signup.regionGlobal')}</span>
          <svg
            className={`w-4 h-4 text-muted transition-transform duration-200 ${regionOpen ? 'rotate-180' : ''}`}
            viewBox="0 0 16 16"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <polyline points="4 6 8 10 12 6" />
          </svg>
        </button>
        {regionOpen && (
          <div
            className="absolute z-50 mt-1 w-full rounded-md border border-ink/10 bg-surface shadow-lg overflow-hidden"
            role="listbox"
            aria-label={t(locale, 'account.region')}
          >
            {REGION_OPTIONS.map((opt) => {
              const selected = region === opt.value;
              return (
                <button
                  key={opt.value}
                  type="button"
                  role="option"
                  aria-selected={selected}
                  data-region-option
                  onClick={() => {
                    onRegionChange(opt.value);
                    setRegionOpen(false);
                    setRegionMsg(true);
                    if (timerRef.current !== null) window.clearTimeout(timerRef.current);
                    timerRef.current = window.setTimeout(() => setRegionMsg(false), 3000);
                  }}
                  onKeyDown={(e) => {
                    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
                      e.preventDefault();
                      const options = Array.from(document.querySelectorAll<HTMLButtonElement>('[data-region-option]'));
                      const idx = options.indexOf(e.currentTarget);
                      const next = e.key === 'ArrowDown' ? options[idx + 1] : options[idx - 1];
                      next?.focus();
                    } else if (e.key === 'Escape') {
                      setRegionOpen(false);
                      const trigger = document.querySelector<HTMLButtonElement>('[aria-haspopup="listbox"]');
                      trigger?.focus();
                    } else if (e.key === 'Enter' || e.key === ' ') {
                      e.preventDefault();
                      e.currentTarget.click();
                    }
                  }}
                  className={`w-full px-3 py-2 text-sm text-left flex items-center gap-2 transition-colors duration-150 ${
                    selected ? 'text-link font-medium' : 'text-ink hover:bg-ink/5'
                  }`}
                >
                  <span>{t(locale, opt.labelKey)}</span>
                  {selected && (
                    <svg className="w-4 h-4 ml-auto text-success" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                      <polyline points="20 6 9 17 4 12" />
                    </svg>
                  )}
                </button>
              );
            })}
          </div>
        )}
      </div>
      {regionMsg && (
        <p className="mt-2 text-sm text-success" role="status">{t(locale, 'account.regionSaved')}</p>
      )}
    </section>
  );
}
