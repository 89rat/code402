#!/usr/bin/env bash
# Stage 2 differential harness — Rust codec vs official @x402/core codec.
# Usage: bash tests/fuzz/run.sh [N]
# CI: schedule nightly (the corpus is seeded-deterministic; nightly reruns
# catch dependency drift on either side).
set -euo pipefail
cd "$(dirname "$0")/../.."
N="${1:-200}"
cargo build -p m2m-core --example codec_roundtrip
node tests/vectors/gen/differential.mjs "$(pwd)/target/debug/examples/codec_roundtrip.exe" "$N"
