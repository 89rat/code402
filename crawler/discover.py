#!/usr/bin/env python3
"""DIS-1 seed expansion — ingest the Coinbase CDP x402 discovery catalog.

The CDP discovery API is a free, public, structured feed of x402-gated resources:
  GET https://api.cdp.coinbase.com/platform/v2/x402/discovery/resources?limit=20&offset=N

Doctrine (same as crawl.py):
  * Catalog quotes are PUBLIC observations (source="public", kind="quoted").
  * Append-only; raw item JSON hashed and kept.
  * http_status=0 means "catalog-sourced, not probed live" — never confuse the two.
  * Live probes (crawl.py on seeds-external.json) exist to catch catalog-vs-live drift;
    that drift is itself a data product nobody else publishes.

Usage:  python discover.py [--max-pages 50] [--seed-cap 200]
"""
import argparse
import hashlib
import json
import sqlite3
import sys
import urllib.request
import urllib.error
from datetime import datetime, timezone
from pathlib import Path

from crawl import SCHEMA  # reuse the append-only schema

CDP_API = "https://api.cdp.coinbase.com/platform/v2/x402/discovery/resources"
PAGE_SIZE = 20  # API clamps limit to 20 regardless of what we ask
UA = "code402-price-crawler/0.1 (+https://code402.dev; honest quoted-price observations)"
TIMEOUT = 30


def utcnow():
    return datetime.now(timezone.utc).isoformat(timespec="seconds")


def fetch_page(offset):
    url = f"{CDP_API}?limit={PAGE_SIZE}&offset={offset}"
    req = urllib.request.Request(url, headers={"User-Agent": UA, "Accept": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=TIMEOUT) as r:
            return json.loads(r.read().decode())
    except Exception as e:
        print(f"  page @{offset} failed: {e}", file=sys.stderr)
        return None


def main():
    ap = argparse.ArgumentParser()
    here = Path(__file__).parent
    ap.add_argument("--db", default=str(here / "observations.db"))
    ap.add_argument("--out", default=str(here / "seeds-external.json"))
    ap.add_argument("--max-pages", type=int, default=50)   # 50 pages * 20 = 1000 catalog entries
    ap.add_argument("--seed-cap", type=int, default=200)   # live probes stay bounded
    args = ap.parse_args()

    db = sqlite3.connect(args.db)
    db.executescript(SCHEMA)
    ts = utcnow()

    n_rows, n_seen, total = 0, 0, None
    seen_urls = set()
    seed_candidates = []

    for page in range(args.max_pages):
        offset = page * PAGE_SIZE
        d = fetch_page(offset)
        if not d or not d.get("items"):
            break
        if total is None:
            total = (d.get("pagination") or {}).get("total")
            print(f"catalog total: {total} resources")
        for it in d["items"]:
            n_seen += 1
            resource = it.get("resource")
            if not resource:
                continue
            raw = json.dumps(it, sort_keys=True)
            sha = hashlib.sha256(raw.encode()).hexdigest()
            accepts = it.get("accepts") or [{}]
            for a in accepts if isinstance(accepts, list) else [{}]:
                if not isinstance(a, dict):
                    a = {}
                db.execute(
                    """INSERT INTO observations
                       (endpoint_id,url,fetched_at,http_status,kind,source,scheme,network,asset,
                        amount_minor,decimals,pay_to,operator,raw_sha256,raw_json)
                       VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)""",
                    (f"cdp:{resource}", resource, ts, 0, "quoted", "public",
                     a.get("scheme"), a.get("network"), a.get("asset"),
                     str(a.get("amount")) if a.get("amount") is not None else None,
                     None, a.get("payTo"), "external", sha, raw),
                )
                n_rows += 1
            # seed candidate: dedup by URL, prefer entries with bazaar input metadata
            if resource not in seen_urls:
                seen_urls.add(resource)
                info = ((it.get("extensions") or {}).get("bazaar") or {}).get("info") or {}
                method = ((info.get("input") or {}).get("method") or "GET").upper()
                seed_candidates.append({
                    "id": f"ext:{resource}",
                    "url": resource,
                    "method": method if method in ("GET", "POST") else "GET",
                    "operator": "external",
                    "source_url": CDP_API,
                })
        if len(d["items"]) < PAGE_SIZE:
            break

    db.commit()

    seeds = {
        "version": 1,
        "note": ("External seed endpoints discovered via the Coinbase CDP x402 discovery "
                 "catalog. Unpaid probes only. Catalog quotes are already in observations.db "
                 "(http_status=0); live probing measures catalog-vs-live drift."),
        "generated_at": ts,
        "endpoints": seed_candidates[: args.seed_cap],
    }
    Path(args.out).write_text(json.dumps(seeds, indent=2), encoding="utf-8")

    eps = db.execute("SELECT COUNT(DISTINCT endpoint_id) FROM observations").fetchone()[0]
    rows = db.execute("SELECT COUNT(*) FROM observations").fetchone()[0]
    print(f"ingested {n_seen} catalog items -> +{n_rows} public quote observations")
    print(f"wrote {len(seeds['endpoints'])} external seeds -> {args.out}")
    print(f"db total: {rows} rows across {eps} endpoints")
    db.close()
    return 0


if __name__ == "__main__":
    sys.exit(main())
