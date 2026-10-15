# Kshana

**The exact moment.** A microsecond-precision astrology engine for both the Western and
Jyotisha traditions. Rust core, Android first.

*Kshana* (क्षण) is Sanskrit for the instant: the smallest unit of time, the moment that
matters. The engine's job is to know the sky at that instant better than anything else
a person can carry, and to be honest about how well it knows.

## Principles

1. **Time is integer nanoseconds on TAI.** No single `f64` Julian Date anywhere in the
   core; floats appear only at the edge where an ephemeris or a frame routine needs one,
   and then as a two-part JD.
2. **Positions come from JPL kernels directly.** DE440/DE441 read from the SPK files NASA
   publishes, not from a compressed derivative.
3. **Every exact moment is a solved root.** Ingresses, stations, returns, tithi
   boundaries, eclipse contacts: Brent's method on exact instants, with the final
   bracket width reported.
4. **Uncertainty is a first-class value.** The user's birth-time window propagates to
   the ascendant, cusps and divisional charts, and the UI shows what is stable.
5. **Reproducible forever.** Every chart carries a provenance stamp: ephemeris, leap
   second table, IERS bulletin, time-zone database and algorithm versions.
6. **Fail honestly.** A house system that is undefined at a latitude returns an error,
   never a made-up cusp.

## Layout

| Crate | What it holds | Status |
|---|---|---|
| `kshana-time` | `Instant` (i128 ns TAI), UTC with leap seconds, TT, TDB, UT1/ΔT, two-part JD | tested |
| `kshana-solver` | Brent root finder on instants, angle wrapping | tested |
| `kshana-ephem` | DAF/SPK reader, Chebyshev evaluation, body chaining | tested on synthetic kernel; real-kernel test behind `--ignored` |
| `kshana-frames` | ERA, GMST, GAST via the CIO locator and equation of the origins, mean obliquity, full IAU 2000A/B nutation (tables generated from ERFA), IAU 2006 bias-precession matrix | matches ERFA to ≤ 1e-12 rad |
| `kshana-houses` | Asc/MC and ten house systems: Placidus, Koch, Regiomontanus, Campanus, Alcabitius, Morinus, Topocentric, Porphyry, Equal, Whole Sign; honest polar failure; horary cusps | match Swiss Ephemeris to < 1″ |
| `kshana-vedic` | Ayanamsa as a fixed sky direction (Lahiri, Raman, KP, Fagan–Bradley, True Chitra), nakshatra, tithi, yoga, karana, Vimshottari to five levels, KP 249 sub-lords and horary numbers, divisional charts D1–D60, Ashtakavarga | ayanamsa within 0.002″ of Swiss Ephemeris over 1900–2050 |
| `kshana-western` | Aspects with applying/separating, signs, traditional dignities, sect | tested |
| `kshana-sky` | Apparent geocentric and topocentric ecliptic positions: light-time, aberration, IAU 2006 bias-precession, IAU 2000A nutation, ecliptic of date, daily motion, true lunar node | agrees with Skyfield to < 0.5″ on 200 geocentric and 20 topocentric checks |
| `kshana-events` | Exact instants on the real sky: ingresses, aspects, lunations, tithi and nakshatra boundaries, stations, sunrise and sunset, planetary hours, Rahu kala, solved to 1 µs | tested against almanac dates and a hand calculation |
| `kshana-verify` | Golden-vector tests against ERFA, jplephem, Skyfield and Swiss Ephemeris (`tests/golden/`) | frames match ERFA to ≤ 1e-12 rad; positions, houses, ayanamsa and sunrise match the others as recorded in `docs/PRECISION.md` |
| `kshana-cli` | `kshana` binary: full chart from the kernel for a UTC instant and a place | working |
| `kshana-rules` | The prediction layer: facts from a chart; KP significators with node agency, cusp sub-lord promise, timing funnel; Prashna judgment with ruling planets and Moon windows; Parashari dignities, aspects, yogas, per-area assessment, period quality; Parihara remedies; Muhurta window search; birth-time rectification from life events | tested on real and synthetic charts; catalogue in `docs/RULES.md` |
| `kshana-chart` | One instant, one place, the whole chart: placements with KP lords and houses, cusps, nodes, panchanga, provenance stamp | tested; shared by the CLI and the FFI |
| `kshana-ffi` | UniFFI surface: `Engine` object (kernel loaded once), `chart`, `next_events`, `dasha_chain`, `horary_lagna`, panchanga, exact instants | tested through the boundary; Kotlin bindings generate cleanly |

See `docs/ARCHITECTURE.md`, `docs/PRECISION.md` and `docs/ROADMAP.md`.

## Build

```sh
cargo test --workspace            # all unit tests (sky tests run only when data/de421.bsp exists)
./scripts/fetch-ephemeris.sh      # de440s.bsp from NAIF into data/
./scripts/fetch-ephemeris.sh pypi # fallback: de421.bsp + finals2000A.all from PyPI when NAIF is blocked
KSHANA_SPK=data/de421.bsp cargo test --workspace -- --ignored
cargo run -p kshana-cli -- chart --spk data/de421.bsp --utc 2026-10-15T09:02:07.418263 --lat 9.9312 --lon 76.2673
cargo run -p kshana-cli -- next --spk data/de421.bsp --utc 2026-10-15T00:00:00 --days 30 --tz +05:30
cargo run -p kshana-cli -- areas --spk data/de421.bsp --utc 1990-10-29T17:36:00 --lat 9.9312 --lon 76.2673
cargo run -p kshana-cli -- prashna --spk data/de421.bsp --utc 2026-10-15T09:02:07 --lat 9.9312 --lon 76.2673 --area Marriage --number 108 --tz +05:30
cargo run -p kshana-cli -- rectify --spk data/de421.bsp --utc 1990-10-29T17:36:00 --lat 9.9312 --lon 76.2673 --window-min 15 --event Career=2013-07-01T04:00:00 --event Marriage=2018-11-22T05:00:00 --tz +05:30
cargo run -p kshana-cli -- muhurta --spk data/de421.bsp --utc 2026-10-15T00:00:00 --days 7 --lat 9.9312 --lon 76.2673 --activity Marriage --natal-moon 243.2 --tz +05:30
./scripts/check.sh                # the gate: fmt, clippy -D warnings, all tests
./scripts/kotlin-check.sh         # type-check the Android sources without the SDK (Maven artifacts only)
./android/build-engine.sh         # Android .so files + Kotlin bindings
```

## Design

The ten app screens (Now, Chart, Vedic, Moments, Birth, Reading, Life Areas, Prashna,
Parihara, Settings) and the logo are on the Kshana design canvas; the look is a night
ground with one gold accent, Cormorant Garamond for display, Manrope for UI and IBM
Plex Mono for every number. The Android shell in `android/` implements them.
