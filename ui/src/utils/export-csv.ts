// Reusable CSV export utility — generates a Blob, triggers download.

export interface CsvColumn {
  key: string;
  label: string;
}

/** Build a CSV string from columns and rows, then trigger a download. */
export function downloadCsv(filename: string, columns: CsvColumn[], rows: Record<string, unknown>[]): void {
  const header = columns.map((c) => escapeCsv(c.label)).join(',');
  const body = rows.map((row) =>
    columns.map((c) => escapeCsv(String(row[c.key] ?? ''))).join(','),
  ).join('\n');
  const blob = new Blob([`\uFEFF${header}\n${body}`], { type: 'text/csv;charset=utf-8' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

function escapeCsv(value: string): string {
  if (value.includes(',') || value.includes('"') || value.includes('\n')) {
    return `"${value.replace(/"/g, '""')}"`;
  }
  return value;
}
