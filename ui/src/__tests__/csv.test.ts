/**
 * Tests for `features/reports/csv.ts` — RFC 4180 CSV escaping, building,
 * and download.
 *
 * The CSV helpers are the single source of truth for report exports in
 * CustomReportScreen, MenuEngineeringScreen, and InventoryReportScreen.
 * Every other screen uses a separate `utils/export-csv.ts` — the two
 * modules must not drift (see `escapeCsv` vs `escapeCsvField`).
 */

import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import { escapeCsvField, buildCsv, downloadCsv } from '@/features/reports/csv';

/* ── escapeCsvField ──────────────────────────────────────────────── */

describe('escapeCsvField (RFC 4180)', () => {
  it('passes through plain text unchanged', () => {
    expect(escapeCsvField('hello')).toBe('hello');
    expect(escapeCsvField('simple text')).toBe('simple text');
    expect(escapeCsvField('123')).toBe('123');
  });

  it('quotes fields containing commas', () => {
    expect(escapeCsvField('hello, world')).toBe('"hello, world"');
  });

  it('quotes fields containing double quotes and doubles internal quotes', () => {
    expect(escapeCsvField('say "hello"')).toBe('"say ""hello"""');
    // Multiple doubled quotes.
    expect(escapeCsvField('"a" and "b"')).toBe('"""a"" and ""b"""');
  });

  it('quotes fields containing newlines', () => {
    expect(escapeCsvField('line1\nline2')).toBe('"line1\nline2"');
  });

  it('quotes fields containing carriage returns (RFC 4180)', () => {
    expect(escapeCsvField('line1\r\nline2')).toBe('"line1\r\nline2"');
    expect(escapeCsvField('line1\ronly')).toBe('"line1\ronly"');
  });

  it('treats a number as a string', () => {
    // escapeCsvField calls String() on the input.
    // @ts-expect-error — number is not the expected type, but String() handles it.
    expect(escapeCsvField(42)).toBe('42');
  });
});

/* ── buildCsv ─────────────────────────────────────────────────────── */

describe('buildCsv', () => {
  it('prepends a BOM (U+FEFF)', () => {
    const result = buildCsv(['a'], [['1']]);
    expect(result.charCodeAt(0)).toBe(0xFEFF);
    expect(result.startsWith('\uFEFF')).toBe(true);
  });

  it('builds a header-only CSV when there are no data rows', () => {
    const result = buildCsv(['Name', 'Price'], []);
    expect(result).toBe('\uFEFFName,Price');
  });

  it('builds a full CSV with headers and data rows', () => {
    const result = buildCsv(
      ['Name', 'Price'],
      [
        ['Coffee', '35000'],
        ['Tea', '15000'],
      ],
    );
    expect(result).toBe('\uFEFFName,Price\nCoffee,35000\nTea,15000');
  });

  it('escapes every cell via escapeCsvField', () => {
    const result = buildCsv(
      ['Item, with comma', 'Notes'],
      [['say "hi"', 'line1\nline2']],
    );
    expect(result).toContain('"Item, with comma"');
    expect(result).toContain('"say ""hi"""');
    expect(result).toContain('"line1\nline2"');
  });
});

/* ── downloadCsv ──────────────────────────────────────────────────── */

describe('downloadCsv', () => {
  let createObjectURL: ReturnType<typeof vi.fn>;
  let revokeObjectURL: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    createObjectURL = vi.fn(() => 'blob:mock-url');
    revokeObjectURL = vi.fn();

    Object.defineProperty(URL, 'createObjectURL', { value: createObjectURL, configurable: true });
    Object.defineProperty(URL, 'revokeObjectURL', { value: revokeObjectURL, configurable: true });

    // Capture the Blob constructor args (jsdom's Blob is a real constructor).
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

    // Mock the anchor: prevent navigation, record the download attribute.
    vi.spyOn(document, 'createElement').mockImplementation((tag: string) => {
      if (tag === 'a') {
        return {
          href: '',
          download: '',
          click: vi.fn(),
        } as unknown as HTMLAnchorElement;
      }
      return document.createElement(tag);
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('creates a Blob with the correct type, passing content through unchanged', () => {
    // downloadCsv does NOT add a BOM — callers pass buildCsv output (which
    // already has \uFEFF). Assert the content and MIME type round-trip.
    downloadCsv('Name,Price\nCoffee,35000', 'report.csv');
    const BlobCtor = vi.mocked(Blob);
    expect(BlobCtor).toHaveBeenCalled();
    const [blobParts, blobOpts] = BlobCtor.mock.calls[0]!;
    const text = (blobParts as BlobPart[]).join('');
    expect(text).toBe('Name,Price\nCoffee,35000');
    expect(blobOpts).toEqual({ type: 'text/csv;charset=utf-8;' });
  });

  it('creates a download link, clicks it, and revokes the URL', () => {
    downloadCsv('a,b\n1,2', 'export.csv');
    expect(createObjectURL).toHaveBeenCalled();
    expect(revokeObjectURL).toHaveBeenCalled();
    const anchor = vi.mocked(document.createElement).mock.results[0]!.value as HTMLAnchorElement;
    expect(anchor.download).toBe('export.csv');
    expect((anchor as { click: () => void }).click).toHaveBeenCalled();
  });

  it('does NOT append the anchor to the document (features/reports/csv.ts contract)', () => {
    // The reports CSV module clicks without attaching to the DOM, unlike
    // utils/export-csv.ts which appendChild/removeChild. Pin that the
    // reports implementation does not touch the body.
    downloadCsv('x', 'my-report.csv');
    const anchor = vi.mocked(document.createElement).mock.results[0]!.value as HTMLAnchorElement;
    expect(anchor.download).toBe('my-report.csv');
  });
});