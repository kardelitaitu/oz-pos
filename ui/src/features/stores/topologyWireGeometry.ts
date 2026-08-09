/** Pure wire-path geometry shared by the canvas wire layer and the
 *  in-flight preview/ghost math. Kept OUT of the component file so
 *  react-refresh only sees component exports there. */

/** Point at fraction `t` along a polyline, weighted by Manhattan segment
 *  length so a simulation pulse crosses each segment at constant speed
 *  (matching the authored-bend visuals, where bends are always drawn as
 *  straight L segments). */
export function polylinePoint(pts: Array<[number, number]>, t: number): { x: number; y: number } {
  if (pts.length < 2) return { x: pts[0]?.[0] ?? 0, y: pts[0]?.[1] ?? 0 };
  let total = 0;
  for (let i = 1; i < pts.length; i++) {
    total += Math.abs(pts[i]![0] - pts[i - 1]![0]) + Math.abs(pts[i]![1] - pts[i - 1]![1]);
  }
  if (total <= 0) return { x: pts[0]![0], y: pts[0]![1] };
  const target = t * total;
  let acc = 0;
  for (let i = 1; i < pts.length; i++) {
    const seg = Math.abs(pts[i]![0] - pts[i - 1]![0]) + Math.abs(pts[i]![1] - pts[i - 1]![1]);
    if (acc + seg >= target || i === pts.length - 1) {
      const frac = seg === 0 ? 0 : (target - acc) / seg;
      return {
        x: pts[i - 1]![0] + (pts[i]![0] - pts[i - 1]![0]) * frac,
        y: pts[i - 1]![1] + (pts[i]![1] - pts[i - 1]![1]) * frac,
      };
    }
    acc += seg;
  }
  return { x: pts[pts.length - 1]![0], y: pts[pts.length - 1]![1] };
}

/** Cubic Bézier interpolation (unbent wires' default curve). */
export function cubicBezier(
  t: number,
  p0: number,
  p1: number,
  p2: number,
  p3: number,
): number {
  const u = 1 - t;
  return u * u * u * p0 + 3 * u * u * t * p1 + 3 * u * t * t * p2 + t * t * t * p3;
}
