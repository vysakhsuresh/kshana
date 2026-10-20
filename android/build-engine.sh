#!/usr/bin/env bash
# Build the Rust engine for every Android ABI, generate the Kotlin bindings, and copy
# the kernel assets into the app.
#
#   ./android/build-engine.sh [abi ...]      # default: arm64-v8a armeabi-v7a x86_64
#
# Needs: rustup targets for the ABIs, cargo-ndk, and ANDROID_NDK_HOME (or an NDK under
# $ANDROID_HOME/ndk, which is picked up automatically). Works from Git Bash on Windows.
set -euo pipefail
cd "$(dirname "$0")/.."

ABIS=("$@")
[ ${#ABIS[@]} -eq 0 ] && ABIS=(arm64-v8a armeabi-v7a x86_64)
OUT=android/app/src/main/jniLibs
mkdir -p "$OUT" android/app/src/main/assets

if [ -z "${ANDROID_NDK_HOME:-}" ]; then
  SDK="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-${LOCALAPPDATA:+$LOCALAPPDATA/Android/Sdk}}}"
  if [ -n "$SDK" ] && [ -d "$SDK/ndk" ]; then
    ANDROID_NDK_HOME=$(ls -d "$SDK"/ndk/* | sort -V | tail -1)
    export ANDROID_NDK_HOME
  fi
fi
echo "NDK: ${ANDROID_NDK_HOME:-<not set>}"

TARGETS=(); for a in "${ABIS[@]}"; do TARGETS+=(-t "$a"); done
cargo ndk "${TARGETS[@]}" -o "$OUT" build --release -p kshana-ffi

# Bindings are generated from the host build of the library (library mode reads the
# UniFFI metadata embedded in the binary; the host artifact name differs per OS).
cargo build --release -p kshana-ffi
HOST_LIB=""
for f in target/release/libkshana_ffi.so target/release/libkshana_ffi.dylib target/release/kshana_ffi.dll; do
  [ -f "$f" ] && { HOST_LIB=$f; break; }
done
[ -n "$HOST_LIB" ] || { echo "host library not found in target/release" >&2; exit 1; }
cargo run --release -p kshana-ffi --bin uniffi-bindgen -- \
  generate --library "$HOST_LIB" --language kotlin --no-format \
  --out-dir android/app/src/main/java

for f in de421.bsp de440s.bsp finals2000A.all; do
  [ -f "data/$f" ] && cp "data/$f" android/app/src/main/assets/
done
echo "engine built for: ${ABIS[*]}; assets: $(ls android/app/src/main/assets)"
