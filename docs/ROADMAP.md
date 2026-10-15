# Roadmap

**M1 · Time and skeleton (this commit).** Exact instants, leap seconds, two-part JD,
solver, SPK reader, frames, Asc/MC, four house systems, panchanga arithmetic, dasha
layout, FFI surface, Android build script, CI.

**M2 · Real sky.** Fetch DE440s; light-time, aberration, deflection; full IAU 2000A
nutation and P03 precession from data tables; TDB from the kernel's own segment; IERS
`finals2000A` loader for UT1 and polar motion; topocentric correction with WGS84 +
geoid. Golden tests against JPL Horizons for Sun, Moon and planets at 50 epochs.

**M3 · Traditions, verified.** Lahiri and the other ayanamsas checked against IAE and
Swiss Ephemeris to 0.01″; Koch, Regiomontanus, Campanus, Alcabitius, Morinus,
Topocentric; divisional charts D1–D60; KP sub-lords; Ashtakavarga; Shadbala; lots,
profections, zodiacal releasing, firdaria; fixed stars from Gaia DR3.

**M4 · Exact moments.** Event solver wired to the sky: ingresses, stations, returns,
aspects, phases, eclipses (contacts), panchanga boundaries, muhurta windows.
Uncertainty propagation and the rectification workbench.

**M5 · Android.** Compose shell over the five screens from the design canvas; offline
kernel bundled; provenance page; differential self-test on first launch.

**Always.** No number ships without a comparison test, and every disagreement with
another ephemeris is written up in `docs/PRECISION.md`.
