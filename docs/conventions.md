# Conventions, and how to reproduce Jagannatha Hora

Kshana's numbers are pinned to independent references by the golden tests in
`crates/kshana-verify/tests/` (see the table at the end). Where two careful programs still
disagree, it is almost always a *convention* — a choice the classics leave open — not an
error. This page lists every convention that changes a number a user can see, Kshana's
choice, Jagannatha Hora's (JH) default, and how big the difference is, so anyone can set
JH (or Kshana) to match the other.

JH is a Windows program and was not run for this page. Its defaults are taken from its
documented Python port **PyJHora** (`src/jhora/const.py`, github.com/naturalstupid/PyJHora,
read 2026-10-21), whose constants are annotated "From JHora" / "JHora value", and from JH's
own preference dialogs as commonly documented. Each size below is measured by a test in
`crates/kshana-verify/tests/conventions.rs`, which switches Kshana to the JH-like setting and
checks that the result moves the stated way by the stated amount.

| # | Convention | Kshana | JH default | Size of the difference | Test |
|---|---|---|---|---|---|
| 1 | Dasha year | 365.25 days (Julian year) | True sidereal year, 365.256364 d (PyJHora `sidereal_year = 365.256364 # From JHora`, `DHASA_YEAR_DURATION.JHORA_DEFAULT = TRUE_SIDEREAL_YEAR`) | Every boundary after birth is later under JH by 0.006364 d per year elapsed since birth: 0.19 d at 30 years, 0.38 d at 60, 0.76 d at 120. Boundaries before birth move earlier by the same rule. Dasha *lords* never change. | `sidereal_dasha_year_moves_every_later_boundary_later_by_the_stated_amount` |
| 2 | Rahu / Ketu | True (osculating) node | True node (PyJHora `_use_true_nodes_for_rahu_ketu = True`); JH offers mean nodes as an option | Same by default. Set to mean, Rahu moves by up to 1°57′ either way (measured 1950–1999: −1.87° … +1.95°), enough to change Rahu's nakshatra or sign near a boundary. Kshana's mean node is on the true equinox (carries Δψ), as Swiss Ephemeris's `SE_MEAN_NODE`. | `mean_node_differs_from_true_node_by_less_than_two_degrees_either_way` |
| 3 | Ayanamsa | Lahiri (Chitrapaksha), Swiss Ephemeris `SE_SIDM_LAHIRI`: 23°15′00.658″ on 1956-03-21, IAU 2006 precession, on the true equinox of date | Lahiri (Chitrapaksha). JH also offers "True Chitrapaksha" = Kshana's `TrueChitra` | Same by default (Kshana's Lahiri matches Swiss Ephemeris to 0.0014″, 1900–2050). True Chitra is smaller than Lahiri by 32″–79″ over 1900–2050 (Spica's annual aberration swings it within each year); under it every sidereal longitude and cusp is larger by exactly that. | `true_chitra_moves_every_sidereal_longitude_by_minus_the_ayanamsa_change`, `every_ayanamsa_matches_swisseph_1900_to_2050` |
| 4 | Sunrise (panchanga day, hora, Rahu kalam, nazhika) | Upper limb with refraction (centre at −0°50′), as Drik Panchang | JH lets the user choose; the "Hindu" rising (centre on the geometric horizon, Swiss Ephemeris `SE_BIT_HINDU_RISING`) is the Kerala/Tamil almanac choice and is Kshana's `RiseConvention::CentreGeometric` | The centre-geometric rising is later by 3.4–3.7 minutes at Kochi through 2026; more at higher latitudes. A tithi or nakshatra that ends within those minutes after sunrise changes the day's name. | `centre_geometric_sunrise_is_later_by_a_few_minutes_at_kochi` |
| 5 | Moon | Geocentric | Geocentric | None. Kshana's topocentric Moon (off by default) moves it by up to ~1°. | — |
| 6 | Rasi chart and houses | Whole-sign rasi from the sidereal lagna; Placidus cusps on the expert pages | Rasi chart whole-sign; JH's bhava (house) chart uses its own configurable method; PyJHora's default is `bhaava_madhya_method = 1` (K. N. Rao: cusp ± 15°) | Rasi placements are identical. For bhava cusps set both programs to the same system; Kshana's Placidus, Koch, Regiomontanus, Campanus, Alcabitius, Morinus, Topocentric, Porphyry, Equal and Whole-sign cusps match Swiss Ephemeris to 0.004″. | `house_cusps_match_swisseph_for_ten_systems`, `chart_ascendant_and_cusps_match_swisseph` |
| 7 | Time scale | UTC → TAI with the IERS leap seconds; UT1 from IERS `finals2000A` where available, else a ΔT table | Swiss Ephemeris ΔT; UT1 = UTC | UT1 − UTC is under 0.9 s: the sidereal time moves by under 14″, the ascendant by a few tens of arcseconds at most; planets are unaffected. | `chart_ascendant_and_cusps_match_swisseph` (removes it to compare) |
| 8 | Ephemeris | JPL DE421 / DE440s | Swiss Ephemeris files (or Moshier) | Against SE's built-in Moshier ephemeris: under 2″ (Sun), 15″ (Moon), 3″ (planets), 10″ (Pluto); against Skyfield on the same DE421 kernel, 0.001″. JH with the SE data files sits between the two. | `positions_match_swisseph_moshier`, `chart_sidereal_positions_match_swisseph_lahiri` |

## Kuta (marriage matching) table variants

The Ashtakoota tables are pinned against an independent transcription of Muhurta
Chintamani's lists for every pada pair (`ashtakoota_matches_muhurta_chintamani_for_every_pada_pair`).
Two of the eight kutas have published variants; Kshana's choice and the alternative:

* **Gana.** Kshana uses the symmetric table: same gana 6, Deva–Manushya 5, Deva–Rakshasa 1,
  Manushya–Rakshasa 0. Some almanacs (and some software) score by direction, giving 6 when
  the bride is Deva and the groom Manushya, and 0 when the bride is Rakshasa and the groom
  Deva or Manushya. The total can differ by up to 1 point.
* **Vashya.** The five groups (Chatushpada, Manava, Jalachara, Vanachara, Keeta) and their
  sign assignment (Sagittarius and Capricorn split at 15°) are standard, but the 5 × 5
  point table varies between texts by ½–1 point in a few cells. Kshana's is symmetric with
  2 on the diagonal; it is checked for those invariants only.
* **Bhakoot and Nadi exceptions** (cancellations when the Moon-sign lords are friends, or the
  same nakshatra in different signs) are not applied to the score; the score is the plain
  table. Software that applies them will show a higher total for those pairs.

## Vimshottari, exactly

Balance at birth = (unexpired part of the Moon's nakshatra) × (the lord's years), the
"rule of three" every panchanga's ready reckoner prints (BPHS ch. 46). Antardashas run from
the mahadasha's own lord in cycle order, each `maha × own / 120`. The cycle is laid out in
integer nanoseconds from one rounding of the balance, so no error accumulates. The golden
test checks all 9 mahadasha and 81 antardasha boundaries of four charts against a naive
implementation to a millisecond (measured: 1 µs).

To reproduce JH's dates: pass `kshana_vedic::dasha::YEAR_SIDEREAL_NS` as the year.

## What the golden tests pin, and against what

| Test (crates/kshana-verify/tests/) | Reference |
|---|---|
| `golden.rs` | ERFA (SOFA) Earth rotation, nutation, precession; jplephem raw DE421 states; Skyfield apparent places |
| `swisseph.rs` | Swiss Ephemeris 2.10.03: tropical positions, nodes, ten house systems, sunrise, all 16 ayanamsas |
| `chart_pipeline.rs` | The app's own chart path against Swiss Ephemeris: sidereal Lahiri longitudes of ten bodies and both nodes, nakshatra/pada, ARMC, ascendant, cusps; panchanga (tithi, nakshatra, yoga, karana at sunrise, sunrise, and every end time) against Drik Panchang day pages |
| `classics.rs` | Vimshottari (BPHS ch. 46, every maha/antar boundary); Ashtakavarga (BPHS ch. 66 table re-transcribed, classical totals 48/49/39/54/56/52/39 and 337 over 150 charts); Ashtakoota (Muhurta Chintamani, 46 656 Moon pairs, each kuta's maximum 1/2/3/4/5/6/7/8) |
| `conventions.rs` | The JH switches on this page |
