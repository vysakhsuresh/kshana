# data/

Runtime data files. Not committed; fetch them with `scripts/fetch-ephemeris.sh`.

| File | Source | Used for |
|---|---|---|
| `de440s.bsp` | NAIF generic kernels | planetary positions 1849–2150 (production) |
| `de421.bsp` | PyPI `skyfield-data` wheel | fallback kernel 1900–2050 when NAIF is unreachable |
| `finals2000A.all` | IERS (also inside `skyfield-data`) | UT1−UTC and polar motion |
| `naif0012.tls` | NAIF | leap seconds cross-check |

Run the real-kernel tests with an absolute or workspace-relative path:

```sh
KSHANA_SPK=data/de421.bsp cargo test -p kshana-ephem -- --ignored
```
