import type { ReactNode } from 'react';

/**
 * SlideWindow — the chrome frame every hero-carousel slide renders inside.
 *
 * Later the user drops a PNG of each app surface into the content area; for
 * now slides 2–5 render a placeholder caption (passed as children) inside a
 * blank window. The chrome (traffic lights + window title) is intentionally
 * NOT part of the PNG — it stays as live DOM so it scales crisply at any
 * hero width and keeps one consistent look across every slide.
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
    <div className="flex h-full w-full flex-col overflow-hidden rounded-2xl border border-ink/10 bg-gradient-to-br from-[#f8f9fa] via-[#f0f1f3] to-[#e8e9eb] shadow-2xl shadow-black/30">
      {/* Window chrome */}
      <div className="flex items-center gap-2 border-b border-ink/10 bg-white/80 px-4 py-1.5">
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
