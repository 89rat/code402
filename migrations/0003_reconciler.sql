-- 0003_reconciler — RECONCILER-SPEC v1 (reviews/reconciler-spec-v1.md) + amendment.
-- resolution columns on settlements; reconciler run bookkeeping.
ALTER TABLE settlements ADD COLUMN resolution TEXT;
ALTER TABLE settlements ADD COLUMN resolution_tx TEXT;
ALTER TABLE settlements ADD COLUMN resolved_at INTEGER;
ALTER TABLE settlements ADD COLUMN replay_eligible_until INTEGER;
CREATE INDEX idx_settlements_stale ON settlements (status, updated_at);
CREATE TABLE reconciler_runs_v2 (
  id INTEGER PRIMARY KEY,
  started_at INTEGER, finished_at INTEGER,
  scanned INTEGER, resolved_used INTEGER, resolved_canceled INTEGER,
  resolved_expired INTEGER, redriven INTEGER, left_ambiguous INTEGER, error TEXT
);
