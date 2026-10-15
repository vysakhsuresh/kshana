#!/usr/bin/env bash
# Build the Rust engine for every Android ABI and generate the Kotlin bindings.
set -euo pipefail
cd "$(dirname "$0")/.."

ABIS=(arm64-v8a armeabi-v7a x86_64)
OUT=android/app/src/main/jniLibs
mkdir -p "$OUT"

cargo ndk -t "${ABIS[@]}" -o "$OUT" build --release -p kshana-ffi

# Bindings are generated from the host build of the library.
cargo build --release -p kshana-ffi
HOST_LIB=$(ls target/release/libkshana_ffi.{so,dylib} 2>/dev/null | head -1)
cargo run --release -p kshana-ffi --bin uniffi-bindgen -- \
  generate --library "$HOST_LIB" --language kotlin \
  --out-dir android/app/src/main/java

echo "engine built for: ${ABIS[*]}"
