-- 0002_settlements — x402 v2 settlement records (plan-rev3 G2/G3/G7,
-- amended per Stage-0 panel audit 2026-08-19: input binding, payment payload
-- persistence, settlement_pending status, format CHECKs).
-- Apply: wrangler d1 migrations apply code402-ledger-staging [--remote]
--
-- Idempotency is (payer, nonce) — EIP-3009 authorizationState uniqueness is
-- per-authorizer; a UNIQUE(nonce) alone would let an attacker front-insert a
-- victim's observed nonce with their own valid payment and deny the victim
-- service (G3). The Durable Object keyed hash(from ‖ nonce) provides mutual
-- exclusion; this table is the durable record + reconciliation source.
--
-- Division of storage (audit Q1): the DO holds the in-flight claim state
-- machine + alarm lease for fast mutual exclusion; the FULL PaymentPayload
-- (payment_payload column) is persisted HERE at claim time, so a crashed
-- isolate's lease-expired claim can be settle-retried from the durable record
-- alone. updated_at is app-maintained (no trigger) — every UPDATE must set it.

CREATE TABLE settlements(
  id                TEXT PRIMARY KEY,                -- settlement id
  payer             TEXT NOT NULL COLLATE NOCASE,    -- EIP-55 address, lowercase-normalized (NOCASE backstop)
  nonce             TEXT NOT NULL CHECK(length(nonce) = 66),  -- 0x + 64 hex (32 bytes)
  request_id        TEXT NOT NULL,                   -- originating cf-ray / request id
  tool              TEXT NOT NULL,
  input_hash        TEXT NOT NULL,                   -- G2c: binds free re-execution to the ORIGINAL request input
  status            TEXT NOT NULL DEFAULT 'claimed'
                    CHECK(status IN ('claimed','settling','settled','settlement_pending','failed',
                                     'receipt_pending','non_replayable')),
  -- 'claimed'           : DO claim held, /settle not yet attempted
  -- 'settling'          : /settle in flight (lease holder alive)
  -- 'settled'           : SettleResponse received; transaction + network populated
  -- 'settlement_pending': spec §9 non-terminal state — settle broadcast, tx hash
  --                      known (transaction non-empty), confirmation unknown
  --                      (e.g. settle-ok-response-lost). Cron reconciles to
  --                      settled/failed. Distinct from receipt_pending (no record).
  -- 'failed'            : terminal settlement failure (facilitator rejection etc.)
  -- 'receipt_pending'   : money moved on-chain without our record (G2d/G7);
  --                       cron backfills from AuthorizationUsed
  -- 'non_replayable'    : settled but response >256KB — replay path disabled (G10)
  scheme            TEXT NOT NULL CHECK(scheme = 'exact'),
  network           TEXT NOT NULL,                   -- CAIP-2, e.g. 'eip155:84532'
  asset             TEXT NOT NULL,                   -- token contract address
  amount            TEXT NOT NULL,                   -- decimal string, minimal units
  pay_to            TEXT NOT NULL,
  requirement_mac   TEXT,                            -- HMAC stamped requirement (G6)
  payment_payload   TEXT NOT NULL,                   -- full inbound PaymentPayload JSON (G3 crash recovery)
  facilitator_req_id TEXT,
  transaction       TEXT,                            -- SettleResponse.transaction (required once settled/settlement_pending)
  settle_network    TEXT,                            -- SettleResponse.network
  response_body     TEXT,                            -- persisted tool response (G2b replay; NULL if non_replayable)
  response_headers  TEXT,                            -- JSON; includes original PAYMENT-RESPONSE
  payment_response  TEXT,                            -- Base64 PAYMENT-RESPONSE header value for replay
  reexec_count      INTEGER NOT NULL DEFAULT 0 CHECK(reexec_count BETWEEN 0 AND 3),  -- G2c bounded free re-execution
  reexec_window_until TEXT,                          -- claimed_at + 24h
  failure_reason    TEXT,                            -- spec §9 taxonomy constant
  created_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  settled_at        TEXT,
  updated_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE UNIQUE INDEX idx_settlements_payer_nonce ON settlements(payer, nonce);
CREATE INDEX idx_settlements_status ON settlements(status);
CREATE INDEX idx_settlements_request ON settlements(request_id);

-- Reconciliation bookkeeping (G7): cron scans settled vs settlement_pending vs
-- receipt_pending and records divergences for alerting.
CREATE TABLE reconciliation_runs(
  run_id       TEXT PRIMARY KEY,
  started_at   TEXT NOT NULL,
  finished_at  TEXT,                                 -- NULL while running (audit: crashed vs running distinguishable)
  checked      INTEGER NOT NULL,
  backfilled   INTEGER NOT NULL DEFAULT 0,
  divergent    INTEGER NOT NULL DEFAULT 0,
  notes        TEXT
);
