/**
 * RFC 4180 CSV escaping utility.
 * 
 * Rules (RFC 4180):
 * - Fields containing commas, double quotes, or newlines MUST be quoted
 * - Double quotes inside quoted fields are escaped by doubling them ("" -> "")
 * - Fields without special characters may be unquoted
 * - A BOM (U+FEFF) should be prepended for UTF-8 identification
 */

export function escapeCsvField(field: string): string {
  const s = String(field);
  if (s.includes(',') || s.includes('"') || s.includes('\n') || s.includes('\r')) {
    return `"${s.replace(/"/g, '""')}"`;
  }
  return s;
}

/**
 * Convert array of rows (each row is array of cells) to CSV string with BOM.
 * Headers are escaped the same way as data cells.
 */
export function buildCsv(
  headers: readonly string[],
  rows: readonly (readonly string[])[],
): string {
  const headerLine = headers.map(escapeCsvField).join(',');
  const dataLines = rows.map((row) => row.map(escapeCsvField).join(','));
  return '\uFEFF' + [headerLine, ...dataLines].join('\n');
}

/**
 * Trigger CSV download in browser.
 */
export function downloadCsv(
  csvContent: string,
  filename: string,
): void {
  const blob = new Blob([csvContent], { type: 'text/csv;charset=utf-8;' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}