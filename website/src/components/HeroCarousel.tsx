import { useEffect, useRef, useState, type ReactNode } from 'react';
import SlideWindow from './mockups/SlideWindow';
import RestaurantMockup from './mockups/RestaurantMockup';

const SLIDE_IDS = ['restaurant', 'retail', 'kitchen', 'warehouse', 'topology'] as const;
export type SlideId = (typeof SLIDE_IDS)[number];

interface Props {
  /** Localized display labels for each slide, keyed by slide id. */
  labels: Record<SlideId, string>;
  /** Localized short descriptions under each label. */
  descriptions: Record<SlideId, string>;
  /** Localized "screenshot coming soon" caption on placeholder slides. */
  comingSoon: string;
}

const DWELL_MS = 5000;
const AUTO_MS = 700;
const MANUAL_MS = 300;
const N = SLIDE_IDS.length;

/** Slide-presentational content. Slide 1 keeps the rich HTML mockup; the rest are placeholders. */
function slideContent(id: SlideId): ReactNode | undefined {
  if (id === 'restaurant') return <RestaurantMockup />;
  // Placeholder slides — user provides PNGs later.
  return undefined;
}

export default function HeroCarousel({ labels, descriptions, comingSoon }: Props) {
  const [index, setIndex] = useState(0);
  const [transitionMs, setTransitionMs] = useState(AUTO_MS);
  const [paused, setPaused] = useState(false);
  const [reduceMotion, setReduceMotion] = useState(false);
  const stageRef = useRef<HTMLDivElement>(null);

  // ── Detect reduced motion + visibility changes ─────────────────────
  useEffect(() => {
    // SSR guard: window is undefined during Astro render.
    if (typeof window === 'undefined') return;

    const mq = window.matchMedia('(prefers-reduced-motion: reduce)');
    const onMq = () => setReduceMotion(mq.matches);
    onMq();
    mq.addEventListener('change', onMq);

    const onVis = () => setPaused(document.hidden);
    document.addEventListener('visibilitychange', onVis);

    return () => {
      mq.removeEventListener('change', onMq);
      document.removeEventListener('visibilitychange', onVis);
    };
  }, []);

  // ── Auto-advance interval ──────────────────────────────────────────
  useEffect(() => {
    if (reduceMotion || paused) return;
    const id = setInterval(() => {
      setTransitionMs(AUTO_MS);
      setIndex((i) => (i + 1) % N);
    }, DWELL_MS);
    return () => clearInterval(id);
  }, [reduceMotion, paused, index]);

  // ── Manual jump to a specific slide ─────────────────────────────────
  const goTo = (i: number) => {
    setTransitionMs(MANUAL_MS);
    setIndex(i);
  };

  // ── Pause on hover / focus (WCAG 2.2.2) ────────────────────────────
  const handlePauseIn = () => setPaused(true);
  const handlePauseOut = () => setPaused(false);

  const trackStyle: React.CSSProperties = reduceMotion
    ? { transform: `translateX(-${index * 100}%)` }
    : {
        transform: `translateX(-${index * 100}%)`,
        transition: `transform ${transitionMs}ms cubic-bezier(0.2, 0, 0, 1)`,
      };

  const highlightStyle: React.CSSProperties = {
    width: `calc((100% - ${2 * 4}px) / ${N})`, // 4px padding each side
    transform: `translateX(${index * 100}%)`,
    transition: reduceMotion ? 'none' : 'transform 300ms cubic-bezier(0.2, 0, 0, 1)',
  };

  return (
    <div
      ref={stageRef}
      role="group"
      aria-roledescription="carousel"
      aria-label="OZ-POS app screenshots"
      className="relative overflow-hidden rounded-2xl border border-ink/10 bg-gradient-to-br from-[#f8f9fa] via-[#f0f1f3] to-[#e8e9eb] shadow-2xl shadow-black/30"
      style={{ aspectRatio: '1280 / 720' }}
      onMouseEnter={handlePauseIn}
      onMouseLeave={handlePauseOut}
      onFocusCapture={handlePauseIn}
      onBlurCapture={handlePauseOut}
    >
      {/* Track — all 5 slides slide horizontally */}
      <div className="flex h-full w-full" style={trackStyle} aria-hidden="true">
        {SLIDE_IDS.map((id) => (
          <div key={id} className="h-full w-full shrink-0">
            <SlideWindow title={`OZ-POS — ${labels[id]}`} content={slideContent(id)}>
              {/* Shown only when a slide has no content (placeholder PNG window) */}
              {slideContent(id) === undefined && (
                <div className="flex h-full flex-col items-center justify-center gap-2 text-center">
                  <p className="text-lg font-semibold text-ink/70">{labels[id]}</p>
                  <p className="max-w-sm px-6 text-sm text-muted">{descriptions[id]}</p>
                  <p className="mt-2 text-[11px] uppercase tracking-widest text-muted/60">{comingSoon}</p>
                </div>
              )}
            </SlideWindow>
          </div>
        ))}
      </div>

      {/* Pill slider — bottom-center overlay */}
      <div className="absolute bottom-3 left-1/2 z-20 -translate-x-1/2">
        <div className="relative flex rounded-full bg-black/35 p-1 backdrop-blur-sm">
          {/* Sliding highlight */}
          <div
            className="absolute inset-y-1 left-1 rounded-full bg-white/90 shadow-sm"
            style={highlightStyle}
            aria-hidden="true"
          />
          {/* Pill buttons */}
          {SLIDE_IDS.map((id, i) => (
            <button
              key={id}
              type="button"
              onClick={() => goTo(i)}
              aria-current={i === index ? 'true' : undefined}
              aria-label={labels[id]}
              className="relative z-10 flex-1 rounded-full px-3 py-1.5 text-[11px] font-semibold transition-colors duration-200"
              style={{
                color: i === index ? '#0f172a' : 'rgba(255,255,255,0.85)',
              }}
            >
              {labels[id]}
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}