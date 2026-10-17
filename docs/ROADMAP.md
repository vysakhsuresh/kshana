# Roadmap

**M1 · Time and skeleton (this commit).** Exact instants, leap seconds, two-part JD,
solver, SPK reader, frames, Asc/MC, four house systems, panchanga arithmetic, dasha
layout, FFI surface, Android build script, CI.

**M2 · Real sky (nearly done).** Done: DE421 kernel, light-time, aberration, IAU 2006
bias-precession, full IAU 2000A nutation and CIO-based GAST from ERFA tables,
ecliptic of date, daily motion, topocentric observer, true node, IERS loader,
golden-vector verification against ERFA/jplephem/Skyfield, exact-event solver,
`kshana chart` and `kshana next`. Remaining: DE440s when NAIF is reachable;
deflection; full TDB series
nutation and P03 precession from data tables; TDB from the kernel's own segment; IERS
`finals2000A` loader for UT1 and polar motion; topocentric correction with WGS84 +
geoid. Golden tests against JPL Horizons for Sun, Moon and planets at 50 epochs.

**M3 · Traditions (first pass done).** Done: KP 249 sub-lords and horary numbers,
significators with node agency, cusp sub-lord promise, Prashna judgment with ruling
planets and Moon windows; Parashari dignities, drishti, eleven yogas, per-area
assessment, period quality; Parihara; divisional charts D1–D60. Remaining: ayanamsa
values checked against IAE to 0.01″; Koch, Regiomontanus, Campanus, Alcabitius,
Morinus, Topocentric; Ashtakavarga; Shadbala; Hellenistic lots, profections, zodiacal
releasing, firdaria; fixed stars from Gaia DR3; Kemadruma cancellations.

**M4 · Exact moments (mostly done).** Done: ingresses, stations, aspects, lunations,
tithi and nakshatra boundaries, sunrise and sunset, horas, Rahu kala, stability under
a birth-time window. Muhurta search done. Remaining: lunar returns, eclipse contacts, the
rectification workbench.

**M5 · Android (built, runs on device).** Compose shell over every screen on the
design canvas, offline kernel in assets, provenance in Settings. The first Android
Studio build is the verification step; then bundled fonts, a differential self-test on
first launch, and the Play listing.

**M6 · The common app (first pass done).** The Loop: nine plain lives scored at every
instant from running periods, Moon-relative transits with vedha and Ashtakavarga, Tara
bala, the planetary hour and the limbs of the day; a countdown to the next shift; per-life
pages for today, week, month, year with trend, best and lowest stretch and what changes;
expert screens moved under More. Also done: Ask (a life × six questions, always
answered), the remedy dictionary, time travel on the Loop; Explain on every life (each
graha's pull, the exact instant it moves on, the solved gain of every coming change, the
three-year slow line); Plan (a scheduled time read for a purpose, with better hours and
days); Us (the thirty-six-point match, Kuja dosha, the pair this second and over
thirty-six months, questions answered from both charts); the Book (chapters and pages,
every sade sati and dhaiya to the second, Jupiter's passages, the returns). Remaining:
calibration of weights against recorded outcomes, bundled fonts, notifications at
shifts the user chooses, widgets, Malayalam and Hindi copy, DE440s so the Book sees
past 2053.

**Always.** No number ships without a comparison test, and every disagreement with
another ephemeris is written up in `docs/PRECISION.md`.
