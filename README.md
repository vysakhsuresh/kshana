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
| `kshana-houses` | Asc/MC, Whole Sign, Equal, Porphyry, Placidus (iterated), polar failure | tested; Koch, Regiomontanus, Campanus etc. pending |
| `kshana-vedic` | Ayanamsa (Lahiri, Raman, KP, Fagan–Bradley), nakshatra, tithi, yoga, karana, Vimshottari to five levels on exact instants, KP 249 sub-lords and horary numbers | tested; ayanamsa values to be verified against IAE in milestone 3 |
| `kshana-western` | Aspects with applying/separating, signs, traditional dignities, sect | tested |
| `kshana-sky` | Apparent geocentric and topocentric ecliptic positions: light-time, aberration, IAU 2006 bias-precession, IAU 2000A nutation, ecliptic of date, daily motion, true lunar node | agrees with Skyfield to < 0.5″ on 200 geocentric and 20 topocentric checks |
| `kshana-events` | Exact instants on the real sky: ingresses, aspects, lunations, tithi and nakshatra boundaries, stations, solved to 1 µs | tested against almanac dates |
| `kshana-verify` | Golden-vector tests against ERFA, jplephem and Skyfield (`tests/golden/`) | ERA, GMST, GAST, obliquity, precession angles, NPB matrix, 2000A/B nutation all match ERFA to ≤ 1e-12 rad |
| `kshana-cli` | `kshana` binary: full chart from the kernel for a UTC instant and a place | working |
| `kshana-rules` | The prediction layer: facts from a chart, KP four-level significators with node agency, cusp sub-lord promise per life area with cited factors, Vimshottari timing funnel with agreement count | tested on a real chart |
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
./scripts/check.sh                # the gate: fmt, clippy -D warnings, all tests
./android/build-engine.sh         # Android .so files + Kotlin bindings
```

## Design

The five app screens (Now, Chart, Vedic, Moments, The Moment You Arrived) are mocked on
the Kshana design canvas; the look is a night ground with one gold accent, Cormorant
Garamond for display, Manrope for UI and IBM Plex Mono for every number.
