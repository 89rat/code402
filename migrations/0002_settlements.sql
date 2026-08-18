-- 0002_settlements — x402 v2 settlement records (plan-rev3 G2/G3/G7).
-- Apply: wrangler d1 migrations apply code402-ledger-staging [--remote]
--
-- Idempotency is (payer, nonce) — EIP-3009 authorizationState uniqueness is
-- per-authorizer; a UNIQUE(nonce) alone would let an attacker front-insert a
-- victim's observed nonce with their own valid payment and deny the victim
-- service (G3). The Durable Object keyed hash(from ‖ nonce) provides mutual
-- exclusion; this table is the durable record + reconciliation source.

CREATE TABLE settlements(
  id                TEXT PRIMARY KEY,                -- settlement id
  payer             TEXT NOT NULL,                   -- EIP-55 address, lowercase-normalized
  nonce             TEXT NOT NULL,                   -- 0x + 64 hex chars (32 bytes)
  request_id        TEXT NOT NULL,                   -- originating cf-ray / request id
  tool              TEXT NOT NULL,
  status            TEXT NOT NULL DEFAULT 'claimed'
                    CHECK(status IN ('claimed','settling','settled','failed',
                                     'receipt_pending','non_replayable')),
  -- 'claimed'      : DO claim held, /settle not yet attempted
  -- 'settling'     : /settle in flight (lease holder alive)
  -- 'settled'      : SettleResponse received; transaction + network populated
  -- 'failed'       : terminal settlement failure (facilitator rejection etc.)
  -- 'receipt_pending': money moved on-chain without our record (G2d/G7);
  --                   cron reconciliation backfills from AuthorizationUsed
  -- 'non_replayable': settled but response >256KB — replay path disabled (G10)
  scheme            TEXT NOT NULL,                   -- 'exact'
  network           TEXT NOT NULL,                   -- CAIP-2, e.g. 'eip155:84532'
  asset             TEXT NOT NULL,                   -- token contract address
  amount            TEXT NOT NULL,                   -- decimal string, minimal units
  pay_to            TEXT NOT NULL,
  requirement_mac   TEXT,                            -- HMAC stamped requirement (G6)
  facilitator_req_id TEXT,
  transaction       TEXT,                            -- SettleResponse.transaction (required once settled)
  settle_network    TEXT,                            -- SettleResponse.network
  response_body     TEXT,                            -- persisted tool response (G2b replay; NULL if non_replayable)
  response_headers  TEXT,                            -- JSON; includes original PAYMENT-RESPONSE
  payment_response  TEXT,                            -- Base64 PAYMENT-RESPONSE header value for replay
  reexec_count      INTEGER NOT NULL DEFAULT 0,      -- bounded free re-execution (G2c, max 3)
  reexec_window_until TEXT,                          -- claimed_at + 24h
  failure_reason    TEXT,                            -- spec §9 taxonomy constant
  created_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  settled_at        TEXT,
  updated_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE UNIQUE INDEX idx_settlements_payer_nonce ON settlements(payer, nonce);
CREATE INDEX idx_settlements_status ON settlements(status);
CREATE INDEX idx_settlements_request ON settlements(request_id);

-- Reconciliation bookkeeping (G7): cron scans settled vs receipt_pending and
-- records divergences for alerting.
CREATE TABLE reconciliation_runs(
  run_id       TEXT PRIMARY KEY,
  started_at   TEXT NOT NULL,
  checked      INTEGER NOT NULL,
  backfilled   INTEGER NOT NULL DEFAULT 0,
  divergent    INTEGER NOT NULL DEFAULT 0,
  notes        TEXT
);
