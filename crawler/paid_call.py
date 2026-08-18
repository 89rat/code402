#!/usr/bin/env python3
"""paid_call.py — orchestrate one REAL mainnet paid call (self-trade, labeled).

Flow:
  1. read gas + nonce from Base mainnet RPC
  2. payprod   -> fresh EIP-3009 voucher (self-payment $0.005 to our company wallet)
  3. settleprod -> signed EIP-1559 transferWithAuthorization tx
  4. broadcast tx, wait for confirmation
  5. call the endpoint with X-PAYMENT + x-settlement-tx -> expect 200 + receipt
  6. record a kind=settled, source=paid-probe observation (self_trade flagged in raw)

Doctrine: this is a SELF-TRADE for engineering validation. It is never counted
in organic demand stats. Cost: ~$0.005 USDC + ~$0.01 gas, our money to ourselves.

Usage: python paid_call.py [--tool vat-mod97-check] [--dry-run]
"""
import argparse
import json
import os
import subprocess
import sys
import time
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent          # code402/
STAGING = ROOT / ".staging"
KEYGEN = ROOT / "crates" / "keygen"
DB = ROOT / "crawler" / "observations.db"
RPC = "https://mainnet.base.org"
PAYER = "0xD654cD6E272571E1be074c5499Cb20fE855a4729"
API = "https://code402.dev/v1/tools/{tool}/call"
GAS_LIMIT = 120_000
UA = {"User-Agent": "Mozilla/5.0", "Content-Type": "application/json"}

TOOL_INPUTS = {
    "vat-mod97-check": {"input": {"vat_number": "GB123456782"}},
    "company-number-format": {"input": {"company_number": "SC123456"}},
    "context-distill": {"input": {"html": "<html><body><p>paid self-test</p></body></html>"}},
}


def rpc(method, params):
    req = urllib.request.Request(
        RPC, data=json.dumps({"jsonrpc": "2.0", "id": 1, "method": method, "params": params}).encode(),
        headers=UA)
    with urllib.request.urlopen(req, timeout=30) as r:
        out = json.loads(r.read().decode())
    if "error" in out:
        raise RuntimeError(f"RPC {method}: {out['error']}")
    return out["result"]


def run_bin(name, env=None):
    exe = KEYGEN / "target" / "release" / f"{name}.exe"
    e = dict(os.environ)
    if env:
        e.update(env)
    p = subprocess.run([str(exe)], cwd=str(KEYGEN), env=e, capture_output=True, text=True)
    if p.returncode != 0:
        raise RuntimeError(f"{name} failed: {p.stderr.strip()[:400]}")
    return p.stdout.strip()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--tool", default="vat-mod97-check", choices=list(TOOL_INPUTS))
    ap.add_argument("--dry-run", action="store_true",
                    help="sign voucher + tx but do NOT broadcast or call the API")
    args = ap.parse_args()

    print("== 1/6 chain state ==")
    payer_nonce = int(rpc("eth_getTransactionCount", [PAYER, "pending"]), 16)
    block = rpc("eth_getBlockByNumber", ["latest", False])
    base_fee = int(block["baseFeePerGas"], 16)
    prio = 1_000_000                      # 0.001 gwei — Base needs almost nothing
    max_fee = base_fee * 2 + prio
    est_cost_eth = GAS_LIMIT * max_fee / 1e18
    print(f"payer nonce={payer_nonce}  base_fee={base_fee/1e9:.5f} gwei  "
          f"max gas cost ~= {est_cost_eth:.6f} ETH")

    print("== 2/6 sign voucher (payprod) ==")
    print(run_bin("payprod"))

    print("== 3/6 build+sign settlement tx (settleprod) ==")
    print(run_bin("settleprod", {
        "PAYER_NONCE": str(payer_nonce),
        "MAX_FEE_WEI": str(max_fee),
        "PRIO_FEE_WEI": str(prio),
        "GAS_LIMIT": str(GAS_LIMIT),
    }))
    raw_tx = (STAGING / "prod-rawtx.txt").read_text().strip()

    if args.dry_run:
        print("DRY RUN — not broadcasting, not calling API.")
        return 0

    print("== 4/6 broadcast ==")
    tx_hash = rpc("eth_sendRawTransaction", [raw_tx])
    print(f"tx: {tx_hash}")
    receipt = None
    for _ in range(30):
        receipt = rpc("eth_getTransactionReceipt", [tx_hash])
        if receipt:
            break
        time.sleep(2)
    if not receipt or receipt.get("status") != "0x1":
        raise RuntimeError(f"settlement tx failed or unconfirmed: {receipt}")
    print(f"confirmed in block {int(receipt['blockNumber'], 16)}  "
          f"gasUsed={int(receipt['gasUsed'], 16)}")

    print("== 5/6 paid API call ==")
    voucher = (STAGING / "prod-voucher.json").read_text().strip()
    req = urllib.request.Request(
        API.format(tool=args.tool),
        data=json.dumps(TOOL_INPUTS[args.tool]).encode(),
        headers={**UA, "X-PAYMENT": voucher, "x-settlement-tx": tx_hash})
    with urllib.request.urlopen(req, timeout=30) as r:
        status, resp = r.status, r.read().decode()
    print(f"API status: {status}")
    print(resp[:500])

    print("== 6/6 record settled observation ==")
    import sqlite3, hashlib
    raw = json.dumps({
        "self_trade": True,
        "note": "Engineering validation: our payer -> our company wallet. Never count as organic demand.",
        "settlement_tx": tx_hash,
        "api_status": status,
        "api_response": json.loads(resp),
    }, sort_keys=True)
    db = sqlite3.connect(str(DB))
    db.execute(
        """INSERT INTO observations
           (endpoint_id,url,fetched_at,http_status,kind,source,scheme,network,asset,
            amount_minor,decimals,pay_to,operator,raw_sha256,raw_json)
           VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)""",
        (f"code402:{args.tool}", API.format(tool=args.tool),
         datetime.now(timezone.utc).isoformat(timespec="seconds"),
         status, "settled", "paid-probe", "eip3009-facilitated-direct", "eip155:8453",
         "USDC", "5000", 6, "0xdcd0fe977640add2dbe62ca0fb30c63f2fd9fdcf",
         "code402 (us)", hashlib.sha256(raw.encode()).hexdigest(), raw))
    db.commit()
    db.close()
    print(f"settled observation recorded. BaseScan: https://basescan.org/tx/{tx_hash}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
