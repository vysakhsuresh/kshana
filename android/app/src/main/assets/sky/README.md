# sky/ — data for the living sky

## bsc.bin — the Yale Bright Star Catalogue to V 5.0

Obtained on 2026-10-19 from brettonw's JSON mirror of the Yale Bright Star Catalogue, 5th
edition (BSC5): `https://raw.githubusercontent.com/brettonw/YaleBrightStarCatalog/master/bsc5-short.json`
(954 206 bytes, 9 096 records with fields HR, RA, Dec, V, K = colour temperature, C =
constellation, F = Flamsteed, B = Bayer, N = common name). The original catalogue is
Hoffleit & Warren 1991, VizieR V/50. Positions are J2000.

Packed by `scratchpad/tools/pack_bsc.py` (not in the repo): every object with V ≤ 5.0 —
1 630 stars, brightest first — plus the common names of the 64 brightest named stars
(Sirius … Naos). 13 675 bytes.

Format, little-endian, version 1:

    "KSKY"  u8 version=1  u8 reserved  u16 count
    count × 8 bytes:
        u16 ra     J2000 right ascension: deg = v × 360 / 65536   (19.8″ step)
        i16 dec    J2000 declination:     deg = v × 90 / 32767    (9.9″ step)
        u8  mag    V = v / 20 − 2                                  (0.05 mag step)
        u8  temp   colour temperature in hundreds of K, 255 = clamped, 0 = unknown
        u16 hr     Harvard Revised number
    u16 named
    named × (u16 index, u8 len, len bytes ASCII)

Read by `ui/sky/Stars.kt` (`Stars.parse`), precessed to the date with the IAU 2006 angles
and projected for the observer from the engine's local sidereal time.

## Fonts under res/font (not in this folder)

Fetched on 2026-10-19 from the google/fonts repository, all under the SIL Open Font
License 1.1 (OFL.txt in each directory of that repository):

- `manrope_wght.ttf` — `ofl/manrope/Manrope[wght].ttf`, 164 700 bytes, variable 200–800,
  unchanged. sha256 3ae11c49db0455a3cc33e37d380f20fdb8c7f8b41dc07625c177e3d87a9d6ae6.
- `ibmplexmono_regular.ttf` — `ofl/ibmplexmono/IBMPlexMono-Regular.ttf`, 135 580 bytes,
  unchanged. sha256 6a3412f058c7d8dfd9170c41e85ade48e5156ecb89356110ca57a0a27734af46.
- `ibmplexmono_medium.ttf` — `ofl/ibmplexmono/IBMPlexMono-Medium.ttf`, 136 704 bytes,
  unchanged. sha256 a9b4c49bb299e05b5f6c481e7fb5e78943d2793249a0c8874ab574a2d1ea6755.
- `cormorantgaramond_medium.ttf` — made from `ofl/cormorantgaramond/CormorantGaramond[wght].ttf`
  (1 195 560 bytes, sha256 b20b7d9626dd956b2c5e558692ad328b1f19e3275e2782db4fa07670d83f35e0),
  which is over the 1 MB per-asset rule, by fonttools 4.67 `varLib.instancer` at wght = 500
  and `subset` to U+0020–007E, U+00A0–017F, U+2000–206F, U+2190–2193, U+2212 and U+20AC,
  keeping all layout features; 257 568 bytes.
  sha256 1f751f248e2f066817a2f2e5fc06ee9e49d358b175a74aa5193da9c095dda338.

The families are exposed by `ui/sky/Fonts.kt` (`KshanaFonts.serif / sans / mono`), which
falls back to the system families when a file is absent.
