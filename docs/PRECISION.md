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

## What is verified today (milestone 2 in progress)

- Sky pipeline on DE421: apparent Sun longitude agrees with Meeus's independent
  low-precision formula to < 0.01° at four epochs 2000–2040; Sun and Moon longitudes
  agree to < 0.02° at the 2000-01-06 new moon; Mars has negative speed at its 2003
  opposition. Next: Horizons golden files at 0.1″.
- IERS Bulletin A loader reads `finals2000A.all`; UT1−UTC at 2000-01-01 = +0.3555 s.
- Real-kernel reader: Earth–Sun at J2000 = 0.9833 au.
- Nutation is still the 4-term truncation (≈1″). Full IAU 2000A/B is the next item.
- Gravitational deflection not yet applied (< 0.01″ beyond 10° from the Sun).

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
