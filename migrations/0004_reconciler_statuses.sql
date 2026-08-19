-- 0004_reconciler_statuses — RECONCILER-SPEC v1 §2 (reviews/reconciler-spec-v1.md):
-- new terminal statuses 'settled_reconciled', 'failed_canceled', 'failed_expired'.
-- SQLite CHECK constraints are immutable: rebuild the table (same columns,
-- extended status domain), copy, swap, recreate indexes (DROP TABLE drops them).
-- Absorbing-state law is enforced in code by guarded UPDATEs
-- (WHERE status IN (...non-terminal...)); the CHECK is the schema backstop.

CREATE TABLE settlements_r1(
  id                TEXT PRIMARY KEY,
  payer             TEXT NOT NULL COLLATE NOCASE,
  nonce             TEXT NOT NULL CHECK(length(nonce) = 66),
  request_id        TEXT NOT NULL,
  tool              TEXT NOT NULL,
  input_hash        TEXT NOT NULL,
  status            TEXT NOT NULL DEFAULT 'claimed'
                    CHECK(status IN ('claimed','settling','settled','settlement_pending','failed',
                                     'receipt_pending','non_replayable',
                                     'settled_reconciled','failed_canceled','failed_expired')),
  scheme            TEXT NOT NULL CHECK(scheme = 'exact'),
  network           TEXT NOT NULL,
  asset             TEXT NOT NULL,
  amount            TEXT NOT NULL,
  pay_to            TEXT NOT NULL,
  requirement_mac   TEXT,
  payment_payload   TEXT NOT NULL,
  facilitator_req_id TEXT,
  tx_hash          TEXT,
  settle_network    TEXT,
  response_body     TEXT,
  response_headers  TEXT,
  payment_response  TEXT,
  reexec_count      INTEGER NOT NULL DEFAULT 0 CHECK(reexec_count BETWEEN 0 AND 3),
  reexec_window_until TEXT,
  failure_reason    TEXT,
  created_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  settled_at        TEXT,
  updated_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  resolution        TEXT,          -- 'facilitator' | 'reconciled_used' | 'reconciled_canceled' | 'reconciled_expired' (0003)
  resolution_tx     TEXT,
  resolved_at       INTEGER,
  replay_eligible_until INTEGER
);

INSERT INTO settlements_r1 SELECT
  id, payer, nonce, request_id, tool, input_hash, status, scheme, network, asset,
  amount, pay_to, requirement_mac, payment_payload, facilitator_req_id, tx_hash,
  settle_network, response_body, response_headers, payment_response, reexec_count,
  reexec_window_until, failure_reason, created_at, settled_at, updated_at,
  resolution, resolution_tx, resolved_at, replay_eligible_until
FROM settlements;

DROP TABLE settlements;
ALTER TABLE settlements_r1 RENAME TO settlements;

CREATE UNIQUE INDEX idx_settlements_payer_nonce ON settlements(payer, nonce);
CREATE INDEX idx_settlements_status ON settlements(status);
CREATE INDEX idx_settlements_request ON settlements(request_id);
CREATE INDEX idx_settlements_stale ON settlements (status, updated_at);
