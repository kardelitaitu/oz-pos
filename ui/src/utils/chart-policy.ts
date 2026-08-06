/**
 * Chart rendering policy (PERF-09).
 *
 * Canvas charts scale poorly at high data cardinality: the line chart draws
 * one arc + stroke per point, the pie chart draws one path + label per slice,
 * and the accessible data list renders one DOM node per datum. This module
 * centralises the limits so every chart bounds its work before drawing and
 * keeps its accessible representation synchronized with the same cap (the
 * A11Y-09 data list must not grow without bound either).
 *
 * All helpers are pure and O(cap) in the worst case — even a 100k-point
 * dataset is downsampled in a bounded number of steps (see the fixture
 * benchmark in `ui/src/__tests__/chart-policy.test.ts`).
 */

/** Maximum points drawn on a line chart before stride downsampling. */
export const CHART_MAX_POINTS = 400;

/** Maximum slices drawn on a pie chart before the tail merges into "Other". */
export const CHART_MAX_SLICES = 12;

/** Maximum items rendered in the accessible sr-only data list. */
export const CHART_ACCESSIBLE_MAX_ITEMS = 100;

/** Any chart datum that carries a numeric value. */
export interface ChartValue {
  value: number;
}

/**
 * Downsample a line dataset to at most `max` points using uniform stride
 * sampling. The first and last points are always preserved so the trend
 * endpoints stay exact; the output is deduplicated because rounding can
 * produce repeated indices at awkward ratios.
 */
export function boundLinePoints<T extends ChartValue>(
  data: readonly T[],
  max: number = CHART_MAX_POINTS,
): T[] {
  if (data.length <= max) return data.slice();
  if (max <= 0) return [];
  // Degenerate cap: a single point — keep the trend endpoint (last value),
  // which is the most informative for a line chart. Skipping this branch
  // would let the "preserve last" push below exceed the cap.
  if (max === 1) return [data[data.length - 1]!];

  const step = (data.length - 1) / (max - 1);
  const sampled: T[] = [];
  for (let i = 0; i < max; i++) {
    sampled.push(data[Math.round(i * step)]!);
  }

  // Deduplicate consecutive identical references (rounding at low stride).
  const deduped: T[] = [];
  for (const p of sampled) {
    if (deduped[deduped.length - 1] !== p) deduped.push(p);
  }
  // Ensure the final point survives (the loop always picks index n-1 via
  // round((max-1)*step), but guard against any edge rounding).
  const last = data[data.length - 1];
  if (last !== undefined && deduped[deduped.length - 1] !== last) deduped.push(last);
  return deduped;
}

/**
 * Keep the top `max - 1` slices by value and aggregate the rest into a
 * single "Other" bucket. Returns the kept slices plus the aggregated
 * remainder so the caller can append a localized "Other" slice with the
 * merged value.
 *
 * With `max === 1` a single largest slice is kept and everything else is
 * merged — never an empty canvas for a non-empty dataset.
 */
export function boundPieSlices<T extends ChartValue>(
  slices: readonly T[],
  max: number = CHART_MAX_SLICES,
): { slices: T[]; otherValue: number } {
  if (slices.length <= max) return { slices: slices.slice(), otherValue: 0 };

  const keep = Math.max(1, max - 1);
  const sorted = slices.slice().sort((a, b) => b.value - a.value);
  const top = sorted.slice(0, keep);
  const tail = sorted.slice(keep);
  const otherValue = tail.reduce((sum, s) => sum + s.value, 0);
  return { slices: top, otherValue };
}

/**
 * Cap a list at `max` items for the accessible data list, reporting how many
 * items were omitted. The charts render their sr-only `<li>` list from this
 * bounded output so a 100k-row report cannot balloon the DOM.
 */
export function boundAccessibleItems<T>(
  items: readonly T[],
  max: number = CHART_ACCESSIBLE_MAX_ITEMS,
): { items: T[]; omitted: number } {
  if (items.length <= max) return { items: items.slice(), omitted: 0 };
  return { items: items.slice(0, max), omitted: items.length - max };
}
