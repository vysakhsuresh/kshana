# The life trace

A person's life, in Kshana, is a sequence of exact instants at which some astrological
state changes. The engine does not sample time and interpolate; it solves each
boundary as a root (Brent's method on integer-nanosecond instants) or derives it by
exact integer arithmetic from the birth instant. The trace is generated lazily for any
window from birth to age 120 and merged into one ordered list.

## What changes, and how each instant is found

| Change | How many per day (typical) | Method | Resolution |
|---|---|---|---|
| Vimshottari level 1–5 (maha → prana) | prana: one every 1–6 days | exact integer split of the birth instant: end = start + total × Σyears / 120 | exact (0 ns) |
| Level 6–9 (deha and below) | level 6 hourly, level 9 every few minutes | same arithmetic, nine deep | exact |
| Moon enters sign / nakshatra / pada / KP sub / KP sub-sub | 0.4 / 1 / 4 / 9 / 80 | crossing solver on the sidereal longitude, bisected until one boundary per piece, then Brent | 1 µs |
| Sun, Mercury, Venus, Mars, Jupiter, Saturn enter sign / nakshatra / pada / sub | days to years apart | same, with retrograde handling (the cell entered is the lower one) | 1 µs |
| Rahu, Ketu (mean node) enter sign / nakshatra | months apart | same, always retrograde | 1 µs |
| Tithi, karana, yoga begin | 1, 2, 1 | crossing solver on Moon − Sun and Moon + Sun | 1 µs |
| Sunrise, sunset | 1, 1 | altitude root with refraction −0°50′ | 1 ms numerically; see below |
| Horas | 24 | twelfths of the day and of the night | follows sunrise |

The 249 KP subs are the Vimshottari-proportioned divisions of each nakshatra, with the
six that straddle a sign boundary split in two. The 2 241 sub-subs divide each listed
sub again in the same proportion from its own lord, the convention of KP software that
lists sub-subs within each sub.

## What "exact" means, honestly

Two different things are being claimed, and the app keeps them apart.

**Numerical resolution.** Every solved instant is bracketed to within 1 µs of the root
of the function the engine evaluates, and every dasha boundary is integer arithmetic.
That is the resolution the trace shows, and it is reproducible to the nanosecond on any
machine.

**Physical accuracy.** The function itself depends on the ephemeris and the time
scales, which have their own limits. The table below converts those limits into time
for the bodies that move slowest to fastest; this is how close the real sky's event is
to the instant shown.

| Source of error | Size | Effect on a Moon event | Sun | Jupiter | Saturn |
|---|---|---|---|---|---|
| JPL DE421 geocentric position (conservative) | Moon 0.05″, Sun 0.05″, outer planets 1″ | 0.3 s | 4 s | 3.5 min | 9 min |
| Interpolated nutation (10-minute samples) | < 1e-7″ | < 1 ms | — | — | — |
| ΔT (TT − UT1) for dates past the IERS table | grows ~0.1 s per year ahead | affects wall-clock labels only, not the sky's own order | | | |
| Ayanamsa model (anchored to Swiss Ephemeris values) | 0.002″ over 1900–2050 | 13 ms | 0.2 s | 8 s | 20 s |
| Sunrise: refraction and the horizon | 20 s typical, more at high latitude | — | | | |

So: a dasha boundary is exact by definition; a Moon sub-sub entry is right to a few
tenths of a second in the real sky and shown to the microsecond; a Saturn sign ingress
is right to about ten minutes in the real sky and shown to the microsecond. The app
prints the numerical resolution beside each instant and this table in Trust.

## Performance (release build, this machine)

| Window | Transitions | Time |
|---|---|---|
| one day, dasha depth 9, Moon to sub-sub, planets to sub, panchanga, horas | 6 449 | 47 ms |
| one week, depth 7 | 1 489 | 0.24 s |
| one month, depth 5 | 3 904 | 1.1 s |
| one year, depth 3 | 47 719 | 13 s |

A phone is roughly three times slower. The app generates a day at a time on demand and
keeps the days it has shown; dasha transitions for a whole life cost milliseconds.

## Interfaces

* Rust: `kshana_trace::{trace, snapshot, dasha_transitions, crossings, Division}`.
* CLI: `kshana trace --spk data/de421.bsp --utc <window start> --birth <birth UTC> --natal-moon <sidereal deg> --lat --lon --days 1 --depth 6 --tz +05:30`.
* FFI: `Engine.trace(...)` and `Engine.snapshot(...)`, `dasha_level_names()`.
