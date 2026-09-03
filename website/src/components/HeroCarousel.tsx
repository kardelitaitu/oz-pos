import { useEffect, useRef, useState, type ReactNode } from 'react';
import SlideWindow from './mockups/SlideWindow';
import RestaurantMockup from './mockups/RestaurantMockup';

const SLIDE_IDS = ['restaurant', 'retail', 'kitchen', 'warehouse', 'topology'] as const;
export type SlideId = (typeof SLIDE_IDS)[number];

interface Props {
  /** Localized display labels for each slide, keyed by slide id. */
  labels: Record<SlideId, string>;
  /** Localized short descriptions under each slide. */
  descriptions: Record<SlideId, string>;
  /** Localized "screenshot coming soon" caption on placeholder slides. */
  comingSoon: string;
}

const DWELL_MS = 10000;
const AUTO_MS = 700;
const MANUAL_MS = 400;
const N = SLIDE_IDS.length;

/** Slide-presentational content. Slide 1 keeps the rich HTML mockup; the rest are placeholders. */
function slideContent(id: SlideId): ReactNode | undefined {
  if (id === 'restaurant') return <RestaurantMockup />;
  return undefined;
}

export default function HeroCarousel({ labels, descriptions, comingSoon }: Props) {
  const [index, setIndex] = useState(0);
  const [transitionMs, setTransitionMs] = useState(AUTO_MS);
  const [paused, setPaused] = useState(false);
  const [reduceMotion, setReduceMotion] = useState(false);
  const [tick, setTick] = useState(0);
  const stageRef = useRef<HTMLDivElement>(null);

  // ── Detect reduced motion + visibility changes ─────────────────────
  useEffect(() => {
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
  }, [reduceMotion, paused, index, tick]);

  // ── Manual jump to a specific slide ─────────────────────────────────
  const goTo = (i: number) => {
    setTransitionMs(MANUAL_MS);
    if (i === index) {
      setTick((t) => t + 1);
      return;
    }
    setIndex(i);
  };

  // ── Pause on hover / focus (WCAG 2.2.2) ────────────────────────────
  const handlePauseIn = () => setPaused(true);
  const handlePauseOut = () => setPaused(false);

  const trackStyle: React.CSSProperties = {
    transform: `translateX(-${index * 100}%)`,
    transition: reduceMotion ? 'none' : `transform ${transitionMs}ms cubic-bezier(0.2, 0, 0, 1)`,
  };

  const highlightStyle: React.CSSProperties = {
    width: `calc((100% - ${2 * 4}px) / ${N})`,
    transform: `translateX(${index * 100}%)`,
    transition: reduceMotion ? 'none' : 'transform 300ms cubic-bezier(0.2, 0, 0, 1)',
  };

  return (
    <div className="flex flex-wrap items-center justify-center gap-12">
      {/* Stage — overflow-hidden container hosting the horizontal track */}
      <div
        ref={stageRef}
        role="group"
        aria-roledescription="carousel"
        aria-label="OZ-POS app screenshots"
        className="relative w-full max-w-[1280px] overflow-hidden rounded-2xl"
        style={{ aspectRatio: '1280 / 720' }}
      >
        {/* Continuous horizontal track: all slides sit side-by-side and glide horizontally */}
        <div className="flex h-full w-full" style={trackStyle}>
          {SLIDE_IDS.map((id, i) => {
            const isActive = i === index;
            return (
              <div
                key={id}
                data-slide-id={id}
                className="h-full w-full shrink-0"
                aria-hidden={!isActive}
              >
                <SlideWindow title={`OZ-POS — ${labels[id]}`} content={slideContent(id)}>
                  {slideContent(id) === undefined && (
                    <div className="flex h-full flex-col items-center justify-center gap-2 text-center">
                      <p className="text-lg font-semibold text-ink/70">{labels[id]}</p>
                      <p className="max-w-sm px-6 text-sm text-muted">{descriptions[id]}</p>
                      <p className="mt-2 text-[11px] uppercase tracking-widest text-muted/60">
                        {comingSoon}
                      </p>
                    </div>
                  )}
                </SlideWindow>
              </div>
            );
          })}
        </div>
      </div>

      {/* Pill slider — outside the mockup, centered beneath it.
          Hovering the controls pauses auto-slide so a user aiming at a
          pill isn't raced by the timer; leaving resumes. */}
      <div
        className="relative flex rounded-full bg-black/35 p-1 backdrop-blur-sm"
        onMouseEnter={handlePauseIn}
        onMouseLeave={handlePauseOut}
      >
        <div
          className="absolute inset-y-1 left-1 rounded-full bg-white/90 shadow-sm"
          style={highlightStyle}
          aria-hidden="true"
        />
        {SLIDE_IDS.map((id, i) => (
          <button
            key={id}
            type="button"
            onClick={() => goTo(i)}
            aria-current={i === index ? 'true' : undefined}
            aria-label={labels[id]}
            className="relative z-10 flex-1 whitespace-nowrap rounded-full px-4 py-2 text-sm font-semibold transition-colors duration-200"
            style={{
              color: i === index ? '#0f172a' : 'rgba(255,255,255,0.85)',
            }}
          >
            {labels[id]}
          </button>
        ))}
      </div>
    </div>
  );
}