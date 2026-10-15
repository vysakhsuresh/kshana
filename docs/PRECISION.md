# Precision and error budget

"Microsecond precision" is a claim about **time arithmetic and event solving**. The
positions themselves are bounded by the ephemeris and by Earth-orientation data. This
page says what each layer contributes, so the app can display an honest error bar.

## Time

| Quantity | Method | Error |
|---|---|---|
| Civil UTC → TAI | integer leap-second table | 0 |
| TAI → TT | +32.184 s by definition | 0 |
| TT → TDB | truncated Fairhead–Bretagnon (scaffold) | ≈ 30 µs → full series in M2: < 1 ns |
| UTC → UT1 (1962–now) | IERS Bulletin A | ≈ 0.1 ms measured; ≈ 1–10 ms for forecasts |
| ΔT before 1962 | tables + polynomials | seconds to minutes, shown as σ |
| Local time → UTC | IANA tzdb, LMT before standard time | 0 where rules are known; flagged otherwise |

A 1 ms error in UT1 moves the Ascendant by about 0.015″. A 4-minute error in birth time
moves it by about 1°. The second dominates every real chart, which is why the UI leads
with the user's uncertainty, not the engine's.

## Positions (DE440, geocentric apparent)

| Body | Ephemeris accuracy | Motion per µs |
|---|---|---|
| Moon | few cm (LLR era) ≈ 0.00002″ | 0.0000006″ |
| Sun / inner planets | sub-km ≈ 0.001″ | 0.00000004″ |
| Outer planets | km to tens of km ≈ 0.001–0.01″ | negligible |

Frame rotation with full IAU 2006/2000A adds < 0.001″. Light-time, aberration and
deflection, done properly, add < 0.001″.

## Event solving

`kshana-solver` returns the instant and the final bracket half-width. Solving to 1 µs
is cheap (≈ 10 evaluations) and meaningful for returns, ingresses, stations and
panchanga boundaries, where the relevant angular rate is well above the ephemeris noise.
For slow bodies the reported bracket is widened to the ephemeris-limited value rather
than quoting a false microsecond.

## What is verified today, against independent reference implementations

Golden vectors in `tests/golden/` are generated offline by `scripts/golden/generate.py`
from ERFA 2.0.1 (the C port of SOFA), jplephem 2.24 and Skyfield 1.55, and replayed by
`crates/kshana-verify`.

| Quantity | Reference | Epochs | Agreement |
|---|---|---|---|
| ERA, GMST, GAST | ERFA `era00`, `gmst06`, `gst06a` | 24, 1950–2050 | ≤ 1e-12 rad |
| Mean obliquity, FW precession angles | ERFA `obl06`, `pfw06` | 24 | ≤ 1e-15 rad |
| Bias-precession-nutation matrix | ERFA `pnm06a` | 24 | ≤ 1e-14 |
| Nutation IAU 2000A (+2006 rate), 2000B | ERFA `nut06a`, `nut00b` | 24 | ≤ 1e-13 rad |
| SPK states, 12 segments | jplephem on DE421 | 12 | ≤ 1e-4 km, ≤ 1e-9 km/s (float noise) |
| Apparent geocentric ecliptic lon/lat, 10 bodies | Skyfield `apparent().ecliptic_latlon(epoch='date')` | 20 | < 0.5″ |
| Topocentric Moon from Kochi | Skyfield `wgs84.latlon` | 20 | < 0.5″ |
| New Moon 2000-01-06, March equinox 2026, Mars stations 2003 | Astronomical Almanac dates | | < 1 min, < 2 min, exact dates |
| Sunrise and sunset, Kochi, 2026-03-20 | hand calculation (equation of time, −0°50′ altitude) | | < 3 min of the estimate; solver itself to 1 ms |
| Ayanamsa, Lahiri, Raman, Krishnamurti, Fagan–Bradley, true equinox | Swiss Ephemeris 2.10.03 `get_ayanamsa_ex_ut` | 16, 1900–2050 | ≤ 0.002″ |
| House cusps, ten systems, five latitudes | Swiss Ephemeris `houses_ex` | 12 charts | < 1″ (Placidus and Koch undefined inside the polar circle in both) |
| Apparent positions, ten bodies | Swiss Ephemeris Moshier mode | 12, 1950–2050 | within Moshier's own accuracy (≤ 3″ planets, ≤ 15″ Moon); the Skyfield/DE comparison above is the tighter one |
| Sunrise and sunset, Kochi and London | Swiss Ephemeris `rise_trans` (default convention) | 3 dates | < 20 s |
| Solar return 2026 for a 1990-10-29 birth | Sun returns to the natal longitude | | < 1e-6° at the solved instant |

## The Android shell

The Kotlin sources (ten screens, profile store, navigation, generated bindings) are
type-checked with `scripts/kotlin-check.sh` against the real Compose 1.7 and Android
framework classes with the Compose compiler plugin loaded. The APK is not built here
because the SDK host is unreachable; the first Android Studio build remains the final
verification step.

Remaining known omissions: gravitational light deflection (≤ 1.75″ at the solar limb,
usually < 0.02″), the full TDB−TT series (≈ 30 µs in time, negligible in angle), and
polar motion in the topocentric observer (< 0.3″).

## Ayanamsa, precisely

Each ayanamsa is defined here as a fixed direction on the sky: the sidereal zero point,
anchored at 2000-01-01T12:00 UTC to the Swiss Ephemeris true-equinox value and carried to
the date with the same IAU 2006/2000A precession-nutation used for the planets. The
result is on the true equinox of date, the frame of the apparent longitudes it is
subtracted from. The earlier mean-equinox approximation differed from this by up to 17″
depending on the year (the nutation in longitude); the comparison table above records
the new agreement.

## Earlier items

- Time layer: J2000 identity, leap-second day 2016-12-31, 3 000-step round trips.
- Solver: exact-nanosecond root on a linear function, microsecond root on a sine.
- Ephemeris reader: synthetic Type 2 kernel; real-kernel smoke test behind `--ignored`.
- Frames: ERA and GMST at J2000, obliquity at J2000.
- Houses: Placidus degenerates to equal-RA at the equator; opposite cusps; polar error.

## What is not yet verified (see ROADMAP)

Nutation beyond 1″, ayanamsa values against the Indian Astronomical Ephemeris, any
position against JPL Horizons. Nothing in the app may show a number from a layer that
has not passed its comparison test.
