-- Gift Cards feature: issue, redeem, top-up, freeze
--
-- gift_cards stores each card's current balance and status.
-- gift_card_transactions records every operation against a card.

CREATE TABLE IF NOT EXISTS gift_cards (
    id                      TEXT PRIMARY KEY,
    card_number             TEXT UNIQUE NOT NULL,
    pin                     TEXT NOT NULL DEFAULT '',
    initial_balance_minor   INTEGER NOT NULL DEFAULT 0,
    current_balance_minor   INTEGER NOT NULL DEFAULT 0,
    currency                TEXT NOT NULL DEFAULT 'IDR',
    status                  TEXT NOT NULL DEFAULT 'active'
                            CHECK (status IN ('active', 'frozen', 'redeemed', 'expired')),
    issued_to               TEXT NOT NULL DEFAULT '',
    issue_date              TEXT NOT NULL,
    expiry_date             TEXT,
    created_by              TEXT REFERENCES users(id),
    updated_at              TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS gift_card_transactions (
    id                  TEXT PRIMARY KEY,
    gift_card_id        TEXT NOT NULL REFERENCES gift_cards(id),
    sale_id             TEXT REFERENCES sales(id),
    txn_type            TEXT NOT NULL CHECK (txn_type IN ('issue', 'redeem', 'topup', 'refund')),
    amount_minor        INTEGER NOT NULL,
    balance_after_minor INTEGER NOT NULL,
    notes               TEXT NOT NULL DEFAULT '',
    created_at          TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_gift_cards_card_number ON gift_cards(card_number);
CREATE INDEX IF NOT EXISTS idx_gift_cards_status ON gift_cards(status);
CREATE INDEX IF NOT EXISTS idx_gift_card_transactions_gift_card_id ON gift_card_transactions(gift_card_id);
CREATE INDEX IF NOT EXISTS idx_gift_card_transactions_sale_id ON gift_card_transactions(sale_id);
