// ── Cart panel width constants ──────────────────────────────────────
// Extracted from RetailCartPanel.tsx to satisfy react-refresh/only-export-components.

export const RETAIL_CART_WIDTH_MIN = 280;
export const RETAIL_CART_WIDTH_DEFAULT = 340;
export const RETAIL_CART_WIDTH_MAX_CAP = 800;

export function clampRetailCartWidth(px: number, viewportWidth: number): number {
  const max = Math.max(
    RETAIL_CART_WIDTH_MIN,
    Math.min(viewportWidth * 0.5, RETAIL_CART_WIDTH_MAX_CAP),
  );
  return Math.max(RETAIL_CART_WIDTH_MIN, Math.min(Math.round(px), max));
}
