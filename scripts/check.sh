#!/usr/bin/env bash
# The gate before every push: format, lint with warnings denied, every test.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
echo "check: all green"
