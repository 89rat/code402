#!/usr/bin/env python3
"""DIS-1 x402 price crawler — code402 intelligence core.

Doctrine (from orphan-bootstrap-plan):
  * A 402 challenge is a PUBLIC price quote. We never pay; unpaid probes only.
  * Every observation is labeled: kind = quoted | settled, source = self-probe | public | paid-probe.
  * Append-only. Raw response hash kept for tamper-evidence. Methodology is published; data is sacred.
  * Never mix our own endpoints' stats into "organic demand" claims.

Usage:  python crawl.py [--seeds seeds.json] [--db observations.db] [--quiet]
"""
import argparse
import base64
import hashlib
import json
import sqlite3
import sys
import urllib.request
import urllib.error
from datetime import datetime, timezone
from pathlib import Path

UA = "code402-price-crawler/0.1 (+https://code402.dev; honest quoted-price observations)"
TIMEOUT = 20

SCHEMA = """
CREATE TABLE IF NOT EXISTS observations (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  endpoint_id   TEXT NOT NULL,
  url           TEXT NOT NULL,
  fetched_at    TEXT NOT NULL,           -- UTC ISO-8601
  http_status   INTEGER NOT NULL,
  kind          TEXT NOT NULL,           -- quoted | settled | error | free
  source        TEXT NOT NULL,           -- self-probe | public | paid-probe
  scheme        TEXT,                    -- exact | upto | batch-settlement | code402-intent | manifest
  network       TEXT,
  asset       TEXT,
  amount_minor  TEXT,                    -- string: big integers stay exact
  decimals      INTEGER,
  pay_to        TEXT,
  operator      TEXT,
  raw_sha256    TEXT NOT NULL,
  raw_json      TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_obs_ep_time ON observations(endpoint_id, fetched_at);
CREATE INDEX IF NOT EXISTS idx_obs_time ON observations(fetched_at);
"""


def utcnow():
    return datetime.now(timezone.utc).isoformat(timespec="seconds")


def http_probe(url, method="GET", body=None):
    """One unpaid probe. Returns (status, headers, raw_bytes). Never raises on HTTP errors."""
    data = None
    headers = {"User-Agent": UA, "Accept": "application/json"}
    if body is not None:
        data = json.dumps(body).encode()
        headers["Content-Type"] = "application/json"
    req = urllib.request.Request(url, data=data, headers=headers, method=method)
    try:
        with urllib.request.urlopen(req, timeout=TIMEOUT) as resp:
            return resp.status, dict(resp.headers), resp.read()
    except urllib.error.HTTPError as e:
        return e.code, dict(e.headers), e.read()
    except Exception as e:
        return -1, {}, json.dumps({"transport_error": str(e)}).encode()


def b64json(s):
    try:
        pad = "=" * (-len(s) % 4)
        return json.loads(base64.b64decode(s + pad))
    except Exception:
        return None


def extract_quotes(status, headers, raw):
    """Tolerant quote extraction. Yields dicts of quote fields.

    Shapes handled:
      - code402 challenge body: {price:{amount,decimals,asset,token_address}, network:{name}, recipient}
      - x402 v2: PAYMENT-REQUIRED header (base64 json) or body with accepts[]
      - x402 v1: body {paymentRequirements: [...]} or {accepts: [...]}
      - manifest (200): {.well-known/x402.json default_price}
    """
    out = []
    body = None
    try:
        body = json.loads(raw.decode("utf-8", "replace"))
    except Exception:
        body = None

    if isinstance(body, dict) and isinstance(body.get("price"), dict):
        p = body["price"]
        out.append({
            "scheme": "code402-intent",
            "network": (body.get("network") or {}).get("name"),
            "asset": p.get("asset"),
            "amount_minor": str(p.get("amount")) if p.get("amount") is not None else None,
            "decimals": p.get("decimals"),
            "pay_to": body.get("recipient"),
        })

    accepts = []
    if isinstance(body, dict):
        accepts = body.get("accepts") or body.get("paymentRequirements") or []
    pr_header = headers.get("PAYMENT-REQUIRED") or headers.get("payment-required")
    if not accepts and pr_header:
        h = b64json(pr_header)
        if isinstance(h, dict):
            accepts = h.get("accepts") or h.get("paymentRequirements") or []
    for a in accepts if isinstance(accepts, list) else []:
        if not isinstance(a, dict):
            continue
        out.append({
            "scheme": a.get("scheme"),
            "network": a.get("network"),
            "asset": a.get("asset") or a.get("assetAddress"),
            "amount_minor": str(a.get("amount") or a.get("maxAmountRequired")) if (a.get("amount") or a.get("maxAmountRequired")) is not None else None,
            "decimals": None,
            "pay_to": a.get("payTo") or a.get("pay_to"),
        })

    if isinstance(body, dict) and status == 200 and "default_price" in body:
        dp = body.get("default_price") or {}
        out.append({
            "scheme": "manifest",
            "network": (body.get("network") or {}).get("name"),
            "asset": body.get("asset"),
            "amount_minor": str(dp.get("amount")) if dp.get("amount") is not None else None,
            "decimals": body.get("decimals"),
            "pay_to": body.get("recipient"),
        })
    return out


def main():
    ap = argparse.ArgumentParser()
    here = Path(__file__).parent
    ap.add_argument("--seeds", default=str(here / "seeds.json"))
    ap.add_argument("--db", default=str(here / "observations.db"))
    ap.add_argument("--quiet", action="store_true")
    args = ap.parse_args()

    seeds = json.loads(Path(args.seeds).read_text(encoding="utf-8"))["endpoints"]
    db = sqlite3.connect(args.db)
    db.executescript(SCHEMA)

    ts = utcnow()
    n_rows = 0
    for ep in seeds:
        status, headers, raw = http_probe(ep["url"], ep.get("method", "GET"), ep.get("body"))
        sha = hashlib.sha256(raw).hexdigest()
        raw_txt = raw.decode("utf-8", "replace")
        quotes = extract_quotes(status, headers, raw)
        if status == 402 and quotes:
            kind = "quoted"
        elif status == 402:
            kind = "quoted"   # 402 but unparseable shape — still a paywall signal
            quotes = [{}]
        elif status == 200:
            kind = "quoted" if quotes else "free"
            if not quotes:
                quotes = [{}]
        else:
            kind = "error"
            quotes = [{}]
        for q in quotes:
            db.execute(
                """INSERT INTO observations
                   (endpoint_id,url,fetched_at,http_status,kind,source,scheme,network,asset,
                    amount_minor,decimals,pay_to,operator,raw_sha256,raw_json)
                   VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)""",
                (ep["id"], ep["url"], ts, status, kind,
                 "self-probe" if ep.get("operator", "").startswith("code402") else "public",
                 q.get("scheme"), q.get("network"), q.get("asset"), q.get("amount_minor"),
                 q.get("decimals"), q.get("pay_to"), ep.get("operator"), sha, raw_txt),
            )
            n_rows += 1
        if not args.quiet:
            price = next((q.get("amount_minor") for q in quotes if q.get("amount_minor")), "-")
            print(f"{status:>4}  {ep['id']:<36} amount={price}  sha={sha[:12]}")
    db.commit()

    total = db.execute("SELECT COUNT(*) FROM observations").fetchone()[0]
    eps = db.execute("SELECT COUNT(DISTINCT endpoint_id) FROM observations").fetchone()[0]
    print(f"\n+{n_rows} observations @ {ts} | db total: {total} rows across {eps} endpoints")
    db.close()
    return 0


def run(ctx):
    """Blueprint Automation entry: daily collection run. Returns the artifact wrapper."""
    import io
    import contextlib
    here = Path(__file__).parent
    buf = io.StringIO()
    argv = sys.argv
    sys.argv = ["crawl.py", "--seeds", str(here / "seeds.json"),
                "--db", str(here / "observations.db")]
    try:
        with contextlib.redirect_stdout(buf):
            main()
    finally:
        sys.argv = argv
    out = buf.getvalue()
    db = sqlite3.connect(str(here / "observations.db"))
    total = db.execute("SELECT COUNT(*) FROM observations").fetchone()[0]
    eps = db.execute("SELECT COUNT(DISTINCT endpoint_id) FROM observations").fetchone()[0]
    db.close()
    return {"artifact": {
        "summary": out.strip().splitlines()[-1] if out.strip() else "no output",
        "db_total": total,
        "endpoints": eps,
        "detail": out.strip(),
    }}


if __name__ == "__main__":
    sys.exit(main())
