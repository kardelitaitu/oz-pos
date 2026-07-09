/// Gift card barcode format: GC-XXXXXXXXXXXX
/// Where X is an alphanumeric character (uppercase hex or digits).
const GIFT_CARD_RE = /^GC-[A-Z0-9]{8,16}$/i;

/**
 * Check whether a scanned/typed code matches the gift card barcode format.
 */
export function isGiftCardBarcode(code: string): boolean {
  return GIFT_CARD_RE.test(code.trim());
}

/**
 * Generate a random gift card number in the format GC-XXXXXXXX.
 */
export function generateGiftCardNumber(): string {
  const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789';
  let result = 'GC-';
  for (let i = 0; i < 12; i++) {
    result += chars.charAt(Math.floor(Math.random() * chars.length));
  }
  return result;
}
