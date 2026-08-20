/**
 * Tests for `utils/export-csv.ts` — the Analytics/Dashboard CSV exporter.
 *
 * This module is the SECOND CSV implementation in the codebase (the other
 * is features/reports/csv.ts). These tests pin its contract AND guard the
 * two against drifting: a regression that removed CR handling here was the
 * original bug (escapeCsv missed \r while escapeCsvField handled it), so
 * the CR case is asserted explicitly.
 */

import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import { downloadCsv, escapeCsv, type CsvColumn } from '@/utils/export-csv';

/* ── escapeCsv (RFC 4180 parity with features/reports/csv.ts) ────── */

describe('escapeCsv', () => {
  it('passes through plain text unchanged', () => {
    expect(escapeCsv('hello')).toBe('hello');
    expect(escapeCsv('simple text')).toBe('simple text');
  });

  it('quotes fields containing commas', () => {
    expect(escapeCsv('hello, world')).toBe('"hello, world"');
  });

  it('quotes fields containing double quotes and doubles them', () => {
    expect(escapeCsv('say "hi"')).toBe('"say ""hi"""');
  });

  it('quotes fields containing LF', () => {
    expect(escapeCsv('line1\nline2')).toBe('"line1\nline2"');
  });

  it('quotes fields containing CR (the original drift bug)', () => {
    // regression: escapeCsv previously missed \r while
    // features/reports/csv.ts::escapeCsvField handled it.
    expect(escapeCsv('line1\r\nline2')).toBe('"line1\r\nline2"');
    expect(escapeCsv('line1\ronly')).toBe('"line1\ronly"');
  });
});

/* ── downloadCsv ──────────────────────────────────────────────────── */

describe('downloadCsv', () => {
  let createObjectURL: ReturnType<typeof vi.fn>;
  let revokeObjectURL: ReturnType<typeof vi.fn>;
  let appendChild: ReturnType<typeof vi.fn>;
  let removeChild: ReturnType<typeof vi.fn>;

  const columns: CsvColumn[] = [
    { key: 'name', label: 'Name' },
    { key: 'note', label: 'Notes' },
  ];

  beforeEach(() => {
    createObjectURL = vi.fn(() => 'blob:mock-url');
    revokeObjectURL = vi.fn();
    appendChild = vi.fn();
    removeChild = vi.fn();

    Object.defineProperty(URL, 'createObjectURL', { value: createObjectURL, configurable: true });
    Object.defineProperty(URL, 'revokeObjectURL', { value: revokeObjectURL, configurable: true });

    vi.spyOn(document, 'createElement').mockImplementation((tag: string) => {
      if (tag === 'a') {
        return { href: '', download: '', click: vi.fn() } as unknown as HTMLAnchorElement;
      }
      return document.createElement(tag);
    });
    vi.spyOn(document.body, 'appendChild').mockImplementation((node: Node) => {
      (appendChild as (n: Node) => void)(node);
      return node;
    });
    vi.spyOn(document.body, 'removeChild').mockImplementation((child: Node) => {
      (removeChild as (c: Node) => void)(child);
      return child;
    });

    vi.spyOn(globalThis, 'Blob').mockImplementation(function BlobMock(
      this: Blob,
      parts?: BlobPart[],
      options?: BlobPropertyBag,
    ) {
      return {
        parts,
        options,
        size: 0,
        type: options?.type ?? '',
        arrayBuffer: async () => new ArrayBuffer(0),
        slice: () => new Blob() as Blob,
        stream: () => new ReadableStream(),
        text: async () => (parts ?? []).map((p) => String(p)).join(''),
      } as unknown as Blob;
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('builds a CSV with escaped headers and cells, prefixed with a BOM', () => {
    downloadCsv('export.csv', columns, [
      { name: 'Coffee', note: 'extra "strong"' },
      { name: 'Tea', note: 'line1\nline2' },
    ]);
    const blobCalls = vi.mocked(Blob).mock.calls;
    expect(blobCalls.length).toBe(1);
    const [parts] = blobCalls[0]!;
    const csv = (parts as BlobPart[]).join('');
    expect(csv.startsWith('\uFEFF')).toBe(true);
    // Header unescaped (no specials).
    expect(csv).toContain('\uFEFFName,Notes\n');
    // Quote-doubling in a cell.
    expect(csv).toContain('"extra ""strong"""');
    // LF quoting.
    expect(csv).toContain('"line1\nline2"');
  });

  it('escapes a CR-containing cell (regression: previously produced malformed CSV)', () => {
    downloadCsv('export.csv', columns, [{ name: 'X', note: 'a\r\nb' }]);
    const [parts] = vi.mocked(Blob).mock.calls[0]!;
    const csv = (parts as BlobPart[]).join('');
    expect(csv).toContain('"a\r\nb"');
  });

  it('uses the download filename and MIME type', () => {
    downloadCsv('analytics.csv', columns, []);
    const anchor = vi.mocked(document.createElement).mock.results[0]!.value as HTMLAnchorElement;
    expect(anchor.download).toBe('analytics.csv');
    const blobOpts = vi.mocked(Blob).mock.calls[0]![1];
    expect(blobOpts).toEqual({ type: 'text/csv;charset=utf-8' });
  });

  it('appends, clicks, and removes the anchor (this module attaches to the DOM)', () => {
    downloadCsv('x.csv', columns, []);
    expect(appendChild).toHaveBeenCalled();
    expect(removeChild).toHaveBeenCalled();
    const anchor = vi.mocked(document.createElement).mock.results[0]!.value as HTMLAnchorElement;
    expect((anchor as { click: () => void }).click).toHaveBeenCalled();
  });

  it('revokes the object URL after download', () => {
    downloadCsv('x.csv', columns, []);
    expect(createObjectURL).toHaveBeenCalled();
    expect(revokeObjectURL).toHaveBeenCalled();
  });
});
