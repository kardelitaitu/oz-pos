import type { ReactNode } from 'react';

/**
 * A11Y-09: visually-hidden chart summary + data list.
 *
 * Canvas charts draw their data into pixels, so screen-reader users only
 * get the canvas `aria-label`. This shared component renders a localized
 * text summary (`summary`) and a list of the underlying values (children)
 * using the global `.sr-only` utility — the data stays out of the visual
 * layout but is fully available to assistive technology.
 *
 * When `summary` is omitted, only the data list renders; when `children`
 * is empty (no data points), nothing renders — the chart still carries its
 * accessible `aria-label`.
 */
export function AccessibleChartSummary({
  summary,
  children,
}: {
  /**
   * Localized text summary of the chart (e.g. "total X across N days").
   * Accepts `undefined` explicitly for `exactOptionalPropertyTypes` — the
   * chart callers always pass their (possibly undefined) summary prop.
   */
  summary: string | undefined;
  /**
   * One `<li>` per data point, with the human-readable value. Optional:
   * the contract is "when `children` is empty (no data points), nothing
   * renders — the chart still carries its accessible `aria-label`".
   */
  children?: ReactNode;
}) {
  const hasItems = Array.isArray(children)
    ? children.some((c) => c !== null && c !== undefined)
    : children !== null && children !== undefined;

  if (!summary && !hasItems) return null;

  return (
    <>
      {summary && <p className="sr-only">{summary}</p>}
      {hasItems && (
        <ul className="sr-only" data-testid="chart-data-list">
          {children}
        </ul>
      )}
    </>
  );
}
