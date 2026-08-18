//! Differential codec roundtrip — Stage 2 G10 harness component.
//! Reads base64 PAYMENT-REQUIRED values (one per line) on stdin; for each:
//! decode via our codec, re-encode, print `OK <b64>` or `ERR <code>`.
//! The TS side (tests/fuzz/differential.mjs) compares against the official
//! @x402/core codec. Run via tests/fuzz/run.sh.

use std::io::{self, BufRead, Write};
use m2m_core::payment::x402v2::{decode_b64_json, encode_b64_json, PaymentRequired};

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                writeln!(out, "ERR io:{e}").ok();
                continue;
            }
        };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match decode_b64_json::<PaymentRequired>(line) {
            Ok(pr) => match encode_b64_json(&pr) {
                Ok(b64) => { let _ = writeln!(out, "OK {b64}"); }
                Err(e) => { let _ = writeln!(out, "ERR encode:{e:?}"); }
            },
            Err(e) => { let _ = writeln!(out, "ERR decode:{e:?}"); }
        }
    }
}
