#!/usr/bin/env bash
# Download the JPL kernels Kshana reads. de440s covers 1849–2150 (≈32 MB);
# de440 covers 1550–2650 (≈114 MB); de441 covers −13200…+17191 (≈3.1 GB).
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p data
BASE=https://naif.jpl.nasa.gov/pub/naif/generic_kernels
KERNEL=${1:-de440s}
curl -fL --retry 4 -o "data/$KERNEL.bsp" "$BASE/spk/planets/$KERNEL.bsp"
curl -fL --retry 4 -o data/naif0012.tls "$BASE/lsk/naif0012.tls"
ls -l data
echo "Run: KSHANA_SPK=data/$KERNEL.bsp cargo test -p kshana-ephem -- --ignored"
