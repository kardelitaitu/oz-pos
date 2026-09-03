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

const DWELL_MS = 5000;
const AUTO_MS = 700;
const MANUAL_MS = 300;
const N = SLIDE_IDS.length;

/** Slide-presentational content. Slide 1 keeps the rich HTML mockup; the rest are placeholders. */
function slideContent(id: SlideId): ReactNode | undefined {
  if (id === 'restaurant') return <RestaurantMockup />;
  return undefined;
}

/**
 * Per-slide transform: active sits at 0; exiting sweeps fully past the
 * left viewport edge; every other slide parks fully past the right
 * viewport edge so the next advance enters from outside the screen.
 * Viewport units (not %) guarantee off-screen on any width — the stage
 * is always inset from the viewport by the centered-layout margin.
 */
function slideTransform(
  i: number,
  index: number,
  leaving: number | null,
): string {
  if (i === index) return 'translateX(0)';
  if (leaving !== null && i === leaving) return 'translateX(-100vw)';
  return 'translateX(100vw)';
}

export default function HeroCarousel({ labels, descriptions, comingSoon }: Props) {
  const [index, setIndex] = useState(0);
  const [leaving, setLeaving] = useState<number | null>(null);
  const [transitionMs, setTransitionMs] = useState(AUTO_MS);
  const [paused, setPaused] = useState(false);
  const [reduceMotion, setReduceMotion] = useState(false);
  const [tick, setTick] = useState(0);
  const stageRef = useRef<HTMLDivElement>(null);
  const leaveTimerRef = useRef<ReturnType<typeof setTimeout>>();
  const prevIndexRef = useRef(index);

  const scheduleClearLeaving = (ms: number) => {
    clearTimeout(leaveTimerRef.current);
    leaveTimerRef.current = setTimeout(() => setLeaving(null), ms);
  };

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

  // Cleanup leave timer on unmount.
  useEffect(() => {
    return () => clearTimeout(leaveTimerRef.current);
  }, []);

  // ── Auto-advance interval ──────────────────────────────────────────
  useEffect(() => {
    if (reduceMotion || paused) return;
    const id = setInterval(() => {
      if (!reduceMotion) {
        setTransitionMs(AUTO_MS);
        setLeaving(prevIndexRef.current);
        scheduleClearLeaving(AUTO_MS + 50);
      }
      setIndex((i) => (i + 1) % N);
    }, DWELL_MS);
    return () => clearInterval(id);
  }, [reduceMotion, paused, index, tick]);

  // Keep prevIndexRef in sync.
  useEffect(() => {
    prevIndexRef.current = index;
  }, [index]);

  // ── Manual jump to a specific slide ─────────────────────────────────
  const goTo = (i: number) => {
    setTransitionMs(MANUAL_MS);
    if (i === index) {
      setTick((t) => t + 1);
      return;
    }
    if (!reduceMotion) {
      setLeaving(index);
      scheduleClearLeaving(MANUAL_MS + 50);
    }
    setIndex(i);
  };

  // ── Pause on hover / focus (WCAG 2.2.2) ────────────────────────────
  const handlePauseIn = () => setPaused(true);
  const handlePauseOut = () => setPaused(false);

  const highlightStyle: React.CSSProperties = {
    width: `calc((100% - ${2 * 4}px) / ${N})`,
    transform: `translateX(${index * 100}%)`,
    transition: reduceMotion ? 'none' : 'transform 300ms cubic-bezier(0.2, 0, 0, 1)',
  };

  return (
    <div className="flex flex-wrap items-center justify-center gap-12">
      {/* Stage — the positioning container (no overflow-hidden, so slides
          entering from off-screen are visible as they sweep across the page). */}
      <div
        ref={stageRef}
        role="group"
        aria-roledescription="carousel"
        aria-label="OZ-POS app screenshots"
        className="relative w-full max-w-[1280px]"
        style={{ aspectRatio: '1280 / 720' }}
        onMouseEnter={handlePauseIn}
        onMouseLeave={handlePauseOut}
        onFocusCapture={handlePauseIn}
        onBlurCapture={handlePauseOut}
      >
        {SLIDE_IDS.map((id, i) => {
          const isActive = i === index;
          const isLeaving = leaving !== null && i === leaving;
          const transform = slideTransform(i, index, leaving);
          const zIndex = isLeaving ? 10 : isActive ? 20 : 0;
          // Only the moving slides animate. Parked slides must NOT carry a
          // transition — otherwise clearing `leaving` would visibly sweep
          // the departed window from the left edge back to its parking
          // spot on the right (ghost sweep across the screen).

          return (
            <div
              key={id}
              data-slide-id={id}
              className="absolute inset-0"
              style={{
                transform,
                zIndex,
                transition:
                  !reduceMotion && (isActive || isLeaving)
                    ? `transform ${transitionMs}ms cubic-bezier(0.2, 0, 0, 1)`
                    : 'none',
              }}
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

      {/* Pill slider — outside the mockup, centered beneath it. */}
      <div className="relative flex rounded-full bg-black/35 p-1 backdrop-blur-sm">
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