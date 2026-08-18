#!/usr/bin/env python3
"""DIS-1 daily pipeline — one entry point for the morning automation.

Sequence (all append-only, all honest labels):
  1. discover.py  — pull the CDP x402 catalog, record public quotes, refresh seeds-external.json
  2. crawl.py     — unpaid live probes of OUR endpoints (seeds.json)
  3. crawl.py     — unpaid live probes of EXTERNAL endpoints (seeds-external.json)

The drift between step 1 (catalog) and step 3 (live) is the data product.

Automation entry: run(ctx) -> {"artifact": {...}}
CLI:  python daily.py [--max-pages 50] [--seed-cap 200]
"""
import argparse
import contextlib
import io
import json
import sqlite3
import sys
from datetime import datetime, timezone
from pathlib import Path

import crawl
import discover
import trust

HERE = Path(__file__).parent
DB = HERE / "observations.db"


def _run_module(mod, argv):
    buf = io.StringIO()
    old = sys.argv
    sys.argv = argv
    try:
        with contextlib.redirect_stdout(buf):
            mod.main()
    finally:
        sys.argv = old
    return buf.getvalue().strip()


def drift_report(db_path):
    """Catalog-vs-live price drift for endpoints probed in the last 48h."""
    db = sqlite3.connect(str(db_path))
    rows = db.execute(
        """SELECT c.url, c.amount_minor, l.amount_minor
           FROM observations c
           JOIN observations l
             ON l.url = c.url AND l.http_status > 0 AND l.amount_minor IS NOT NULL
            AND l.fetched_at > datetime('now', '-2 days')
           WHERE c.http_status = 0 AND c.amount_minor IS NOT NULL
           GROUP BY c.url"""
    ).fetchall()
    same = sum(1 for r in rows if r[1] == r[2])
    drifters = [
        {"url": r[0], "catalog": r[1], "live": r[2]} for r in rows if r[1] != r[2]
    ]
    stats = {
        "total_rows": db.execute("SELECT COUNT(*) FROM observations").fetchone()[0],
        "endpoints": db.execute(
            "SELECT COUNT(DISTINCT endpoint_id) FROM observations"
        ).fetchone()[0],
        "live_402_24h": db.execute(
            """SELECT COUNT(DISTINCT url) FROM observations
               WHERE http_status = 402 AND fetched_at > datetime('now', '-1 day')"""
        ).fetchone()[0],
    }
    db.close()
    return {
        "comparable": len(rows),
        "agree": same,
        "drift_count": len(drifters),
        "drift_rate_pct": round(100 * len(drifters) / len(rows), 1) if rows else None,
        "drifters": drifters[:20],
        **stats,
    }


def pipeline(max_pages=50, seed_cap=200):
    sections = []
    sections.append(("discover", _run_module(
        discover, ["discover.py", "--db", str(DB),
                   "--out", str(HERE / "seeds-external.json"),
                   "--max-pages", str(max_pages), "--seed-cap", str(seed_cap)])))
    sections.append(("crawl:self", _run_module(
        crawl, ["crawl.py", "--seeds", str(HERE / "seeds.json"),
                "--db", str(DB), "--quiet"])))
    sections.append(("crawl:external", _run_module(
        crawl, ["crawl.py", "--seeds", str(HERE / "seeds-external.json"),
                "--db", str(DB), "--quiet"])))
    sections.append(("trust", _run_module(
        trust, ["trust.py"])))
    return sections


def run(ctx):
    """Blueprint Automation entry: full daily collection + drift report."""
    ts = datetime.now(timezone.utc).isoformat(timespec="seconds")
    sections = pipeline()
    drift = drift_report(DB)
    detail = "\n\n".join(f"== {name} ==\n{out}" for name, out in sections)
    summary = (
        f"{ts} | {drift['total_rows']} rows / {drift['endpoints']} endpoints | "
        f"live 402s (24h): {drift['live_402_24h']} | "
        f"drift: {drift['drift_count']}/{drift['comparable']} "
        f"({drift['drift_rate_pct']}%)"
    )
    return {"artifact": {
        "summary": summary,
        "drift": drift,
        "detail": detail,
    }}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--max-pages", type=int, default=50)
    ap.add_argument("--seed-cap", type=int, default=200)
    args = ap.parse_args()
    result = run(None)
    a = result["artifact"]
    print(a["summary"])
    print("\n" + a["detail"])
    if a["drift"]["drifters"]:
        print("\n== drifters ==")
        for d in a["drift"]["drifters"]:
            print(f"  {d['url'][:70]}  catalog={d['catalog']} live={d['live']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
