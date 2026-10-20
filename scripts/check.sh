#!/bin/sh
# SPACE INVADERS — Metal Arcade verification gate.
#
# Run this before every commit, or simply `make check`.
#
# Stages:
#   1. cargo fmt --check
#   2. cargo clippy (all targets, warnings denied)
#   3. cargo test (core unit tests, scene/renderer/audio tests, doctests)
#   4. the standalone SpaceInvadersChecks behavioral suite
#   5. a release build of the whole workspace
#   6. docs-check: Markdown links, heading anchors, and cited Rust symbols
#
# The shell is POSIX so the gate works with /bin/sh; cargo and python3 are the
# only tools it needs beyond the Rust toolchain.

set -eu
cd "$(dirname "$0")/.."

CARGO="${CARGO:-cargo}"
PYTHON="${PYTHON:-python3}"

echo "==> cargo fmt --check"
$CARGO fmt --all -- --check

echo "==> cargo clippy (warnings denied)"
$CARGO clippy --workspace --all-targets --locked -- -D warnings

echo "==> cargo test"
$CARGO test --workspace --locked

echo "==> simulation checks"
$CARGO run --release --locked -p space-invaders-checks

echo "==> release build"
$CARGO build --release --workspace --locked

echo "==> docs-check"
$PYTHON scripts/check_docs.py

echo "==> all checks passed"
