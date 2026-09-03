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

/** Clean, distinct content for each slide to visually confirm sliding. */
function slideContent(id: SlideId, label: string, desc: string): ReactNode {
  return (
    <div className="flex h-full w-full flex-col items-center justify-center gap-4 p-8 text-center bg-gradient-to-br from-slate-100 to-slate-200 dark:from-slate-900 dark:to-slate-800">
      <div className="text-6xl font-extrabold text-blue-600 dark:text-blue-400">
        {id === 'restaurant' && '☕ 1'}
        {id === 'retail' && '🏷️ 2'}
        {id === 'kitchen' && '🍳 3'}
        {id === 'warehouse' && '📦 4'}
        {id === 'topology' && '🌐 5'}
      </div>
      <h2 className="text-3xl font-bold text-slate-900 dark:text-white">{label}</h2>
      <p className="max-w-md text-base text-slate-600 dark:text-slate-300">{desc}</p>
    </div>
  );
}

export default function HeroCarousel({ labels, descriptions, comingSoon }: Props) {
  const [index, setIndex] = useState(0);
  const [transitionMs, setTransitionMs] = useState(AUTO_MS);
  const [paused, setPaused] = useState(false);
  const [tick, setTick] = useState(0);
  const stageRef = useRef<HTMLDivElement>(null);

  // ── Auto-advance timer ─────────────────────────────────────────────
  useEffect(() => {
    if (paused) return;
    const id = setInterval(() => {
      setTransitionMs(AUTO_MS);
      setIndex((i) => (i + 1) % N);
    }, DWELL_MS);
    return () => clearInterval(id);
  }, [paused, index, tick]);

  // ── Manual jump to a specific slide ─────────────────────────────────
  const goTo = (i: number) => {
    setTransitionMs(MANUAL_MS);
    if (i === index) {
      setTick((t) => t + 1);
      return;
    }
    setIndex(i);
  };

  // ── Pause on hover (WCAG 2.2.2) ────────────────────────────────────
  const handlePauseIn = () => setPaused(true);
  const handlePauseOut = () => setPaused(false);

  const trackStyle: React.CSSProperties = {
    transform: `translateX(-${index * 100}%)`,
    transition: `transform ${transitionMs}ms cubic-bezier(0.2, 0, 0, 1)`,
  };

  const highlightStyle: React.CSSProperties = {
    width: `calc((100% - ${2 * 4}px) / ${N})`,
    transform: `translateX(${index * 100}%)`,
    transition: 'transform 300ms cubic-bezier(0.2, 0, 0, 1)',
  };

  return (
    <div className="flex flex-wrap items-center justify-center gap-12">
      {/* Stage — rounded container hosting the horizontal track with deep elevation shadow */}
      <div
        ref={stageRef}
        role="group"
        aria-roledescription="carousel"
        aria-label="OZ-POS app screenshots"
        className="relative w-full max-w-[1280px] overflow-hidden rounded-3xl border border-ink/10 bg-surface/50 shadow-2xl shadow-black/40 backdrop-blur"
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
                className="h-full w-full shrink-0 p-3 sm:p-5"
                aria-hidden={!isActive}
              >
                <SlideWindow title={`OZ-POS — ${labels[id]}`} content={slideContent(id, labels[id], descriptions[id])} />
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