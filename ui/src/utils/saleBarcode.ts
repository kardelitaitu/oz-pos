const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

/** Check whether a scanned/typed code matches the sale receipt UUID format. */
export function isSaleBarcode(code: string): boolean {
  return UUID_RE.test(code.trim());
}
