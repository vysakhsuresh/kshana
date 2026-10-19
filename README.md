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
| `kshana-vedic` | Ayanamsa as a fixed sky direction (Lahiri, Raman, KP, Fagan–Bradley, True Chitra), nakshatra, tithi, yoga, karana, Vimshottari to five levels, Yogini, Ashtottari, Kalachakra and Chara dashas, KP 249 sub-lords and horary numbers, divisional charts D1–D60, Ashtakavarga, Jaimini karakas, naming syllables | ayanamsa within 0.002″ of Swiss Ephemeris over 1900–2050 |
| `kshana-western` | Aspects with applying/separating, signs, traditional dignities, sect | tested |
| `kshana-sky` | Apparent geocentric and topocentric ecliptic positions: light-time, aberration, IAU 2006 bias-precession, IAU 2000A nutation, ecliptic of date, daily motion, true lunar node | agrees with Skyfield to < 0.5″ on 200 geocentric and 20 topocentric checks |
| `kshana-events` | Exact instants on the real sky: ingresses, aspects, lunations, tithi and nakshatra boundaries, stations, sunrise and sunset, planetary hours, Rahu kala, solved to 1 µs | tested against almanac dates and a hand calculation |
| `kshana-verify` | Golden-vector tests against ERFA, jplephem, Skyfield and Swiss Ephemeris (`tests/golden/`) | frames match ERFA to ≤ 1e-12 rad; positions, houses, ayanamsa and sunrise match the others as recorded in `docs/PRECISION.md` |
| `kshana-cli` | `kshana` binary: chart, next, areas, prashna, muhurta, rectify, life, match, calendar, trace, dashas, varsha, directions | working |
| `kshana-rules` | The prediction layer: facts from a chart; KP significators with node agency, cusp sub-lord promise, timing funnel; Prashna judgment with ruling planets and Moon windows; Parashari dignities, aspects, yogas, per-area assessment, period quality; Parihara remedies; Muhurta window search; birth-time rectification from life events; Ashtakoota matching, Mangal dosha, Western synastry | tested on real and synthetic charts; catalogue in `docs/RULES.md` |
| `kshana-forecast` | What the sky does to one chart: sign transits with exact sidereal ingress instants, Sade Sati phases, gochara scoring (Phaladeepika vedha, Moorthi Nirnaya, Ashtakavarga bindus, kakshya windows), the day's good and avoid hours, the 100-year life map, the observance calendar | tested against the kernel |
| `kshana-trace` | The life trace: every dasha level to nine deep and every zodiac crossing (sign, nakshatra, pada, KP sub, sub-sub), tithi, karana, yoga, hora, sunrise and sunset as exact instants; snapshots at any instant | tested against snapshots; `docs/TRACE.md` |
| `kshana-ledger` | The life ledger: dated events scored against the running periods and transits (favour or adversity by kind), matched baselines at instants nobody logged, Brier and reliability bins, and a retrodiction of every past antardasha | tested on the kernel |
| `kshana-directions` | Western and Hellenistic timing: secondary progressions (day-for-a-year on 365.2422 d), solar arc, annual and monthly profections on exact solar returns, firdaria, zodiacal releasing from Fortune and Spirit with loosing of the bond, lunar and demi returns, exact transit hits to every natal point | 24 tests; every instant a solved root |
| `kshana-tajika` | Tajika varshaphala: the exact sidereal solar return, Muntha, the year lord by Panchavargeeya bala, 35 Sahams, Ithasala, Ishrafa, Nakta, Yamaya, Manau and Kamboola, Mudda dasha | 19 tests; Charak and Neelakanthi cited |
| `kshana-speak` | Plain words: a reading in sentences, each carrying its factors, confidence and rule ids; compatibility in words; a 56-term glossary | tested |
| `kshana-chart` | One instant, one place, the whole chart: placements with KP lords and houses, cusps, nodes, panchanga, provenance stamp | tested; shared by the CLI and the FFI |
| `kshana-ffi` | UniFFI surface: `Engine` object (kernel loaded once); chart, events, areas, prashna, parihara, muhurta, rectify, plain reading, life map, daily, transits, compatibility, calendar, karakas, naming, glossary, self-test, export | tested through the boundary; Kotlin bindings generate cleanly |

See `docs/ARCHITECTURE.md`, `docs/PRECISION.md`, `docs/TRACE.md` and `docs/ROADMAP.md`.

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
cargo run -p kshana-cli -- life --spk data/de421.bsp --utc 1990-10-29T17:36:00 --lat 9.9312 --lon 76.2673
cargo run -p kshana-cli -- match --spk data/de421.bsp --utc 1990-10-29T17:36:00 --lat 9.9312 --lon 76.2673 --utc2 1992-04-03T06:15:00 --lat2 12.97 --lon2 77.59
cargo run -p kshana-cli -- calendar --spk data/de421.bsp --utc 2026-10-15T00:00:00 --lat 9.9312 --lon 76.2673 --days 60 --tz +05:30
cargo run -p kshana-cli -- trace --spk data/de421.bsp --utc 2026-10-16T00:00:00 --birth 1990-10-29T12:06:00 --natal-moon 316.344 --lat 9.9312 --lon 76.2673 --days 1 --depth 6 --tz +05:30
cargo run -p kshana-cli -- dashas --spk data/de421.bsp --utc 1990-10-29T12:06:00 --lat 9.9312 --lon 76.2673 --at 2026-10-16T00:00:00 --tz +05:30
cargo run -p kshana-cli -- varsha --spk data/de421.bsp --utc 1990-10-29T12:06:00 --lat 9.9312 --lon 76.2673 --year 2026 --tz +05:30
cargo run -p kshana-cli -- directions --spk data/de421.bsp --utc 1990-10-29T12:06:00 --lat 9.9312 --lon 76.2673 --tz +05:30
./scripts/check.sh                # the gate: fmt, clippy -D warnings, all tests
./scripts/kotlin-check.sh         # type-check the Android sources without the SDK (Maven artifacts only)
./android/build-engine.sh         # Android .so files + Kotlin bindings
```

## The app

Five doors, in the words a person already has:

| Tab | What it answers | Behind it |
|---|---|---|
| **Today** | What is today like for me, which hours are mine, what is the sky doing to my chart | `daily`, `transits`, `next_events` |
| **My life** | Who am I, what is promised, what is running now, the next ten years, the whole life as a map | `plain_reading`, `life_map`, the timing funnel, remedies |
| **Ask** | A yes/no question with a chosen number, judged now | KP Prashna with ruling planets and Moon windows |
| **Right time** | When to start something, the fasting and festival calendar, every exact moment | Muhurta, `calendar`, events, reminders |
| **Me** | My birth, my family, matching two charts, a baby's naming syllable, the glossary, and the instrument (Western chart, Vedic kundli, trust, settings) | `compatibility`, `naming`, `glossary`, charts, self-test, export |

A **Plain words / Astrologer** switch decides how much of the instrument shows. Every
sentence can be opened to show the factors, the confidence and the rule id behind it;
the rules are catalogued in `docs/RULES.md` and nothing is written by a language model.

**Online, only if allowed.** The app is complete offline. When the person switches
online on: any place on Earth can be searched (OpenStreetMap Nominatim), the Earth
rotation data can be refreshed from IERS, and an optional narration of the computed
reading can be requested from a Claude model with the person's own API key, grounded
strictly in the engine's sentences and forbidden from adding claims.

The screens and the logo are on the Kshana design canvas; the look is a night ground
with one gold accent, Cormorant Garamond for display, Manrope for UI and IBM Plex Mono
for every number. The Android shell in `android/` implements them.
