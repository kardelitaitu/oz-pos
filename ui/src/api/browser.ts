// ── Browser opening (ADR #38) ──────────────────────────────────────
//
// `openProductImagesScoped` asks the backend to open the OS default
// browser in a new tab at a Google Images search for the product's
// name (+ brand). The query is built server-side; the frontend only
// passes the SKU.

import { loggedInvoke } from '@/utils/logged-invoke';

/**
 * Open the default browser at a Google Images search for a product
 * (ADR #38 D2/D3: query = name + brand, built server-side).
 *
 * Returns `false` when the opener is unavailable (dev-mock fallback:
 * `window.open`), `true` when the backend accepted the request.
 */
export const openProductImagesScoped = async (
  sessionToken: string,
  sku: string,
): Promise<boolean> => {
  try {
    await loggedInvoke<void>('open_product_images_scoped', { sessionToken, sku });
    return true;
  } catch (err) {
    // Dev-mock / browser fallback: open a Google Images search directly.
    console.warn('open_product_images_scoped unavailable, using window.open fallback', err);
    const url = `https://www.google.com/search?tbm=isch&q=${encodeURIComponent(sku)}`;
    window.open(url, '_blank', 'noopener,noreferrer');
    return false;
  }
};
