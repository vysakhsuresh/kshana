#!/usr/bin/env bash
# Download the kernels Kshana reads.
#   ./scripts/fetch-ephemeris.sh            # de440s from NAIF (≈32 MB, 1849–2150)
#   ./scripts/fetch-ephemeris.sh de440      # 1550–2650, ≈114 MB
#   ./scripts/fetch-ephemeris.sh pypi       # de421 + finals2000A from the skyfield-data wheel on PyPI
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p data
case "${1:-de440s}" in
  pypi)
    tmp=$(mktemp -d)
    pip download --no-deps -d "$tmp" -q skyfield-data
    whl=$(ls "$tmp"/*.whl | head -1)
    python3 -I - "$whl" <<'PY'
import sys, zipfile
z = zipfile.ZipFile(sys.argv[1])
for n in ("skyfield_data/data/de421.bsp", "skyfield_data/data/finals2000A.all"):
    with z.open(n) as src, open("data/" + n.rsplit("/", 1)[1], "wb") as dst:
        dst.write(src.read())
PY
    rm -rf "$tmp"
    ;;
  *)
    BASE=https://naif.jpl.nasa.gov/pub/naif/generic_kernels
    curl -fL --retry 4 -o "data/$1.bsp" "$BASE/spk/planets/$1.bsp"
    curl -fL --retry 4 -o data/naif0012.tls "$BASE/lsk/naif0012.tls"
    ;;
esac
ls -l data
