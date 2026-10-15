#!/usr/bin/env bash
# Build the Rust engine for every Android ABI, generate the Kotlin bindings, and copy
# the kernel assets into the app.
set -euo pipefail
cd "$(dirname "$0")/.."

ABIS=(arm64-v8a armeabi-v7a x86_64)
OUT=android/app/src/main/jniLibs
mkdir -p "$OUT" android/app/src/main/assets

cargo ndk -t "${ABIS[@]}" -o "$OUT" build --release -p kshana-ffi

# Bindings are generated from the host build of the library.
cargo build --release -p kshana-ffi
HOST_LIB=$(ls target/release/libkshana_ffi.so target/release/libkshana_ffi.dylib 2>/dev/null | head -1)
cargo run --release -p kshana-ffi --bin uniffi-bindgen -- \
  generate --library "$HOST_LIB" --language kotlin --no-format \
  --out-dir android/app/src/main/java

for f in de421.bsp de440s.bsp finals2000A.all; do
  [ -f "data/$f" ] && cp "data/$f" android/app/src/main/assets/
done
echo "engine built for: ${ABIS[*]}; assets: $(ls android/app/src/main/assets)"
