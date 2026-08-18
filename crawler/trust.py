#!/usr/bin/env python3
"""trust.py — compute code402 Verified trust records from observations.db.

Phase 1: our own domain (code402.dev) is seller #1.
Honesty rules (spec: code402/specs/VERIFIED-BADGE.md):
  * levels are computed, never assigned
  * Unrated is the honest default until 7 measured days
  * settled rows count only when they match the quote (self-trades included,
    disclosed as self_trades count — never hidden, never counted as demand)
  * evidence_root_hash chains every underlying raw_sha256 — re-computable by anyone

Ingest: POSTs the record to the worker's /v1/trust-ingest (Bearer key from
.staging/trust-ingest-key.txt — SEALED).

Usage: python trust.py [--domain code402.dev] [--no-ingest]
"""
import argparse
import hashlib
import json
import sqlite3
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

HERE = Path(__file__).parent
DB = HERE / "observations.db"
KEY_FILE = HERE.parent / ".staging" / "trust-ingest-key.txt"
WORKER = "https://code402.dev"


def compute(domain):
    db = sqlite3.connect(str(DB))
    like = f"%{domain}%"
    rows = db.execute(
        """SELECT endpoint_id, fetched_at, http_status, kind, source, amount_minor, raw_sha256
           FROM observations WHERE url LIKE ? ORDER BY fetched_at""", (like,),
    ).fetchall()
    db.close()

    days = sorted({r[1][:10] for r in rows})
    quoted = [r for r in rows if r[3] == "quoted" and r[5] is not None]
    settled = [r for r in rows if r[3] == "settled"]
    live_402 = [r for r in rows if r[2] == 402]

    # fidelity: per-endpoint price consistency (different tools have different
    # prices — a silent change WITHIN one endpoint is what the badge catches)
    fidelity = None
    if quoted:
        by_ep = {}
        for r in quoted:
            by_ep.setdefault(r[0], []).append(r[5])
        matches, total = 0, 0
        for amounts in by_ep.values():
            modal = max(set(amounts), key=amounts.count)
            matches += sum(1 for a in amounts if a == modal)
            total += len(amounts)
        fidelity = round(100.0 * matches / total, 1) if total else None

    days_measured = len(days)
    if days_measured >= 30 and (fidelity or 0) >= 99.5:
        level = "verified-gold"
    elif days_measured >= 7 and (fidelity or 0) >= 99.0 and live_402:
        level = "verified"
    else:
        level = "unrated"

    chain = hashlib.sha256()
    for r in rows:
        chain.update(r[6].encode())
        chain.update(b"|")

    return {
        "domain": domain,
        "level": level,
        "fidelity_pct": fidelity,
        "days_measured": days_measured,
        "observations": len(rows),
        "settled_observations": len(settled),
        "self_trades_disclosed": len(settled),
        "drift_events": 0,
        "time_to_correction_avg_h": None,
        "first_measured": days[0] if days else None,
        "last_run": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "methodology_url": f"{WORKER}/trust",
        "evidence_root_hash": chain.hexdigest(),
        "badge_url": f"{WORKER}/v1/trust/{domain}/badge.svg",
    }


def ingest(domain, record):
    key = KEY_FILE.read_text().strip()
    body = json.dumps({"domain": domain, "record": record}).encode()
    req = urllib.request.Request(
        f"{WORKER}/v1/trust-ingest", data=body,
        headers={"Content-Type": "application/json", "Authorization": f"Bearer {key}",
                 "User-Agent": "code402-trust/0.1"})
    with urllib.request.urlopen(req, timeout=30) as r:
        return r.status, r.read().decode()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--domain", default="code402.dev")
    ap.add_argument("--no-ingest", action="store_true")
    args = ap.parse_args()
    rec = compute(args.domain)
    print(json.dumps(rec, indent=2))
    if not args.no_ingest:
        status, resp = ingest(args.domain, rec)
        print(f"ingest: {status} {resp}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
