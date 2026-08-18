-- code402 ledger — append-only discipline; only `status` may mutate.
-- Apply: wrangler d1 migrations apply code402-ledger-staging

CREATE TABLE payment_events(event_id TEXT PRIMARY KEY, request_id TEXT NOT NULL, tool_id TEXT NOT NULL,
 tool_version TEXT NOT NULL, payer TEXT, chain_id INTEGER, token TEXT, tx_hash TEXT, amount_minor INTEGER NOT NULL,
 status TEXT NOT NULL, error_code TEXT, created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')));
CREATE INDEX idx_events_request ON payment_events(request_id);
CREATE TABLE receipts(request_id TEXT PRIMARY KEY, input_hash TEXT NOT NULL, output_hash TEXT NOT NULL,
 signature TEXT NOT NULL, r2_key TEXT NOT NULL, created_at TEXT NOT NULL);
CREATE TABLE idempotency(idem_key TEXT PRIMARY KEY, request_id TEXT NOT NULL, response_ref TEXT NOT NULL, created_at TEXT NOT NULL);
