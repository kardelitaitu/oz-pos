import type { ReactNode } from 'react';

/**
 * SlideWindow — the chrome frame every hero-carousel slide renders inside.
 *
 * Later the user drops a PNG of each app surface into the content area; for
 * now slides 2–5 render a placeholder caption (passed as children) inside a
 * blank window. The chrome (traffic lights + window title) is intentionally
 * NOT part of the PNG — it stays as live DOM so it scales crisply at any
 * hero width and keeps one consistent look across every slide.
 *
 * Theming: the frame is built from the surface tokens, not literal greys, so
 * it follows `[data-theme="dark"]`. The mapping is deliberately NOT
 * hex-for-hex: the old gradient (#f8f9fa → #e8e9eb) sat between `bg` and
 * `surface`, and the tokens that match those values in light (`ghost-bg`,
 * `chip-bg`) are the ones that go LIGHTER than `surface` in dark, where the
 * ladder inverts. Using them would have made the frame brighter than the
 * cards it contains and swallowed them. `from-bg to-surface/30` keeps the
 * frame strictly below `surface` in both themes, so `bg-surface/*` cards stay
 * raised either way.
 */
export default function SlideWindow({
  title,
  content,
  children,
}: {
  title: string;
  /** Full-height app content (e.g. the rich HTML mockup for slide 1). */
  content?: ReactNode;
  /** Fallback caption shown when `content` is absent (placeholder PNG window). */
  children?: ReactNode;
}) {
  return (
    <div className="flex h-full w-full flex-col overflow-hidden rounded-2xl border border-ink/10 bg-gradient-to-br from-bg to-surface/30">
      {/* Window chrome */}
      <div className="flex items-center gap-2 border-b border-ink/10 bg-surface/70 px-4 py-1.5">
        <span className="h-2.5 w-2.5 rounded-full bg-red-400" />
        <span className="h-2.5 w-2.5 rounded-full bg-yellow-400" />
        <span className="h-2.5 w-2.5 rounded-full bg-green-400" />
        <span className="ml-4 truncate text-[11px] font-medium text-muted">{title}</span>
      </div>
      {/* App content — rich mockup, a future PNG, or the placeholder caption */}
      <div className="min-h-0 flex-1">{content ?? children ?? null}</div>
    </div>
  );
}
