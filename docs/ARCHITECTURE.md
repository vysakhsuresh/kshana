# Architecture

```
          ┌──────────────────────────────────────────────────────┐
          │                    Android (Kotlin)                   │
          │   Compose UI · provenance display · uncertainty UI    │
          └───────────────▲───────────────────────────────────────┘
                          │ UniFFI (generated Kotlin)
          ┌───────────────┴───────────────────────────────────────┐
          │                    kshana-ffi                          │
          └───────────────▲───────────────────────────────────────┘
                          │
          ┌───────────────┴───────────────────────────────────────┐
          │   kshana-speak (words) · kshana-forecast (transits,    │
          │   daily, life map, calendar) · kshana-rules (KP,       │
          │   Parashari, Prashna, Muhurta, Parihara, matching)     │
          │                   over kshana-chart                    │
          └───────────────▲───────────────────────────────────────┘
                          │
   ┌──────────────┬───────┴────────┬──────────────┬──────────────┐
   │ kshana-vedic │ kshana-western │ kshana-houses│ kshana-solver│
   └──────▲───────┴───────▲────────┴──────▲───────┴──────▲───────┘
          │               │               │              │
   ┌──────┴───────────────┴───────────────┴──────────────┴───────┐
   │                 kshana-frames  ·  kshana-ephem               │
   └──────────────────────────────▲──────────────────────────────┘
                                  │
                          ┌───────┴───────┐
                          │  kshana-time  │
                          └───────────────┘
```

## Data flow for one chart

1. **Civil input** (date, local time, place, uncertainty window) → IANA time zone rules →
   `Utc` → `Instant` (exact, TAI nanoseconds). A DST gap or overlap is an error the UI
   must resolve with the user, never a silent choice.
2. `Instant` → two-part **TDB JD** → `kshana-ephem` evaluates DE440 Chebyshev records →
   barycentric ICRF state vectors (km, km/s).
3. Geocentric apparent places: light-time iteration, aberration, gravitational
   deflection (milestone 2), then **frame rotation** by precession–nutation to the true
   equator and equinox of date, then to ecliptic longitude.
4. `Instant` + IERS UT1−UTC → **ERA → GAST → LST** at the observer's longitude.
5. LST, true obliquity, latitude → **Ascendant, MC, cusps** (`kshana-houses`).
6. Tropical longitudes → **ayanamsa** → sidereal longitudes → nakshatra, tithi, dashas
   (`kshana-vedic`); aspects, dignities, sect (`kshana-western`).
7. Every "exact moment" the UI shows is `kshana-solver::find_root` over a closure that
   re-runs steps 2–6 at a candidate instant.

## Provenance stamp

Attached to every computed chart:

```
engine        kshana 0.1.0 (git sha)
ephemeris     de440s.bsp  (NAIF, sha256 …)
leap seconds  IERS Bulletin C 68
eop           IERS finals2000A 2026-10-10
tzdb          2026b
ayanamsa      Lahiri (definition v1)
houses        Placidus
```

Same stamp + same input = bit-identical output, on any device, in any year.

## Why Rust

Integer-exact time needs `i128`; the solver and the Chebyshev loops must be fast on a
phone; the same code must run on Android, iOS, the web (WASM) and in the verification
harness (Python bindings). One core, no re-implementations.
