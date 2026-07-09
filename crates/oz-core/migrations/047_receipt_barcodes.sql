CREATE TABLE IF NOT EXISTS receipt_barcodes (
    id          TEXT PRIMARY KEY,
    sale_id     TEXT NOT NULL REFERENCES sales(id),
    barcode     TEXT NOT NULL UNIQUE,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_receipt_barcodes_barcode ON receipt_barcodes(barcode);
CREATE INDEX IF NOT EXISTS idx_receipt_barcodes_sale_id ON receipt_barcodes(sale_id);
