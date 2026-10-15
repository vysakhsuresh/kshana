#!/usr/bin/env python3
"""Generate Swiss Ephemeris golden test vectors for Kshana.

Everything runs OFFLINE and needs NO Swiss Ephemeris data files: the
built-in Moshier ephemeris (``swe.FLG_MOSEPH``) is used for every body
position, the ayanamsa models are analytic, and the house systems need only
sidereal time and obliquity (both computed internally by swisseph).

Setup (once; the same venv as scripts/golden/generate.py can be reused):

    python3 -m venv /path/to/oracle-venv
    /path/to/oracle-venv/bin/pip install pyswisseph

Run (from anywhere; paths are resolved relative to this file):

    /path/to/oracle-venv/bin/python scripts/golden/generate_swisseph.py

Output (overwritten each run; re-running reproduces it byte for byte):

    tests/golden/swisseph.json

Sections
--------
ayanamsa   16 UTC epochs 1900-2050; for each of SIDM_FAGAN_BRADLEY,
           SIDM_LAHIRI, SIDM_RAMAN, SIDM_KRISHNAMURTI, SIDM_TRUE_CITRA the
           value of ``swe.get_ayanamsa_ut(jd_ut)`` (mean equinox, i.e.
           "without nutation") and of ``swe.get_ayanamsa_ex_ut(jd_ut,
           FLG_MOSEPH)`` (true equinox of date; this is the value swisseph
           subtracts from tropical longitudes when FLG_SIDEREAL is set).
houses     12 (epoch, place) pairs, latitudes -43.5 .. +69.6, ten house
           systems from ``swe.houses_ex(jd_ut, lat, lon, hsys, 0)``
           (tropical).  pyswisseph raises ``swisseph.Error`` where the C
           library would return Porphyry cusps with retc = -1 (Placidus /
           Koch inside the polar circles); such cases are recorded with an
           ``error`` field.  Any successful system whose cusps coincide with
           Porphyry ('O') is flagged ``fell_back_to_porphyry``.
positions  12 UTC epochs 1950-2050, geocentric apparent tropical ecliptic
           longitude / latitude / distance / speeds of Sun .. Pluto, mean
           node and true node from ``swe.calc(jd_tt, body,
           FLG_MOSEPH|FLG_SPEED)``, plus Lahiri sidereal Sun and Moon with
           FLG_SIDEREAL and the ayanamsa swisseph applied.
kp         intentionally empty: swisseph has no KP sub-lord tables.
sunrise    ``swe.rise_trans`` for the Sun at Kochi (2026-03-20, 2026-10-15)
           and London (2026-06-21), rise and set, in three conventions:
           default (upper limb, with refraction), BIT_NO_REFRACTION (upper
           limb, geometric) and BIT_DISC_CENTER (centre, with refraction).

Time scales
-----------
``jd_tt, jd_ut = swe.utc_to_jd(y, m, d, h, mi, s, swe.GREG_CAL)``.
swisseph's UTC->TT uses its own leap-second table (TAI-UTC = 37 s carried
forward after 2017-01-01) and ``jd_ut`` is UT1 = UTC here (swisseph does not
apply UT1-UTC).  For dates before 1972 swisseph treats the input as UT1 and
forms TT = UT + delta-T.  Both JDs are recorded in every record together
with ``delta_t_s`` = ``swe.deltat_ex(jd_ut, FLG_MOSEPH) * 86400`` (Moshier
tidal acceleration, -25.58 "/cy^2).
"""

import json
import os
import platform
import sys

import swisseph as swe

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", ".."))
OUT = os.path.join(REPO, "tests", "golden", "swisseph.json")

POS_FLAGS = swe.FLG_MOSEPH | swe.FLG_SPEED
SID_FLAGS = POS_FLAGS | swe.FLG_SIDEREAL

SIDMODES = [
    ("SIDM_FAGAN_BRADLEY", swe.SIDM_FAGAN_BRADLEY),
    ("SIDM_LAHIRI", swe.SIDM_LAHIRI),
    ("SIDM_RAMAN", swe.SIDM_RAMAN),
    ("SIDM_KRISHNAMURTI", swe.SIDM_KRISHNAMURTI),
    ("SIDM_TRUE_CITRA", swe.SIDM_TRUE_CITRA),
]

BODIES = [
    ("sun", swe.SUN), ("moon", swe.MOON), ("mercury", swe.MERCURY),
    ("venus", swe.VENUS), ("mars", swe.MARS), ("jupiter", swe.JUPITER),
    ("saturn", swe.SATURN), ("uranus", swe.URANUS), ("neptune", swe.NEPTUNE),
    ("pluto", swe.PLUTO), ("mean_node", swe.MEAN_NODE),
    ("true_node", swe.TRUE_NODE),
]

HSYS = ["P", "K", "R", "C", "B", "M", "T", "O", "E", "W"]

PLACES = {
    "kochi": (9.9312, 76.2673),
    "london": (51.5074, -0.1278),
    "reykjavik": (64.1466, -21.9426),
    "sydney": (-33.8688, 151.2093),
    "quito": (-0.1807, -78.4678),
    "christchurch": (-43.5321, 172.6362),
    "new_york": (40.7128, -74.0060),
    "tromso": (69.6492, 18.9553),  # inside the Arctic circle: P/K undefined
}

# (year, month, day, hour, minute, second)
AYANAMSA_EPOCHS = [
    (1900, 1, 1, 0, 0, 0.0), (1910, 6, 15, 12, 0, 0.0),
    (1920, 3, 21, 6, 30, 0.0), (1930, 11, 11, 18, 0, 0.0),
    (1940, 7, 4, 3, 15, 0.0), (1950, 1, 1, 0, 0, 0.0),
    (1960, 9, 23, 9, 0, 0.0), (1970, 12, 25, 21, 45, 0.0),
    (1980, 5, 5, 5, 5, 5.0), (1990, 2, 14, 14, 0, 0.0),
    (2000, 1, 1, 12, 0, 0.0), (2010, 10, 10, 10, 10, 10.0),
    (2020, 6, 21, 0, 0, 0.0), (2026, 10, 15, 9, 30, 0.0),
    (2040, 4, 1, 16, 20, 0.0), (2050, 12, 31, 23, 59, 59.0),
]

POSITION_EPOCHS = [
    (1950, 1, 1, 0, 0, 0.0), (1962, 11, 5, 4, 0, 0.0),
    (1975, 2, 28, 9, 45, 0.0), (1985, 7, 15, 18, 30, 0.0),
    (1999, 12, 31, 23, 59, 59.0), (2000, 1, 1, 12, 0, 0.0),
    (2010, 12, 21, 15, 0, 0.0), (2020, 6, 21, 0, 0, 0.0),
    (2026, 10, 15, 9, 30, 0.0), (2033, 9, 23, 6, 0, 0.0),
    (2042, 3, 20, 21, 15, 0.0), (2050, 12, 31, 23, 0, 0.0),
]

HOUSE_CASES = [
    ("kochi", (2000, 1, 1, 12, 0, 0.0)),
    ("kochi", (2026, 3, 20, 6, 0, 0.0)),
    ("kochi", (1985, 7, 15, 18, 30, 0.0)),
    ("london", (2026, 6, 21, 12, 0, 0.0)),
    ("london", (1962, 11, 5, 4, 0, 0.0)),
    ("reykjavik", (2010, 12, 21, 15, 0, 0.0)),
    ("sydney", (2026, 10, 15, 20, 0, 0.0)),
    ("sydney", (1975, 2, 28, 9, 45, 0.0)),
    ("quito", (2020, 6, 21, 0, 0, 0.0)),
    ("christchurch", (2033, 9, 23, 6, 0, 0.0)),
    ("new_york", (1999, 12, 31, 23, 59, 59.0)),
    ("tromso", (2026, 6, 21, 12, 0, 0.0)),
]

SUNRISE_CASES = [
    ("kochi", (2026, 3, 20)),
    ("kochi", (2026, 10, 15)),
    ("london", (2026, 6, 21)),
]

RISE_VARIANTS = [
    ("default", 0,
     "upper limb, with refraction (swisseph default; ~ Sun's centre at "
     "-0deg50' = 34' refraction + 16' semi-diameter)"),
    ("no_refraction", swe.BIT_NO_REFRACTION,
     "upper limb, geometric (no refraction)"),
    ("disc_center", swe.BIT_DISC_CENTER,
     "disc centre, with refraction (~ centre at -34')"),
]


def iso(y, m, d, h, mi, s):
    if s == int(s):
        return "%04d-%02d-%02dT%02d:%02d:%02dZ" % (y, m, d, h, mi, int(s))
    return "%04d-%02d-%02dT%02d:%02d:%09.6fZ" % (y, m, d, h, mi, s)


def epoch(y, m, d, h, mi, s):
    jd_tt, jd_ut = swe.utc_to_jd(y, m, d, h, mi, s, swe.GREG_CAL)
    return {
        "utc": iso(y, m, d, h, mi, s),
        "jd_ut": jd_ut,
        "jd_tt": jd_tt,
        "delta_t_s": swe.deltat_ex(jd_ut, swe.FLG_MOSEPH) * 86400.0,
    }


def norm360(x):
    x = x % 360.0
    return x + 360.0 if x < 0 else x


def ecl_nut(jd_ut):
    xx, _ = swe.calc_ut(jd_ut, swe.ECL_NUT, swe.FLG_MOSEPH)
    return {
        "true_obliquity_deg": xx[0],
        "mean_obliquity_deg": xx[1],
        "nutation_longitude_deg": xx[2],
        "nutation_obliquity_deg": xx[3],
    }


# ---------------------------------------------------------------- ayanamsa
def gen_ayanamsa():
    recs = []
    for ep in AYANAMSA_EPOCHS:
        e = epoch(*ep)
        vals = {}
        for name, mode in SIDMODES:
            swe.set_sid_mode(mode, 0.0, 0.0)
            aya_mean = swe.get_ayanamsa_ut(e["jd_ut"])
            retflags, aya_true = swe.get_ayanamsa_ex_ut(e["jd_ut"], swe.FLG_MOSEPH)
            vals[name] = {
                "sidmode": mode,
                "name": swe.get_ayanamsa_name(mode),
                "get_ayanamsa_ut_deg": aya_mean,
                "get_ayanamsa_ex_ut_deg": aya_true,
                "get_ayanamsa_ex_ut_retflags": retflags,
            }
        e["ayanamsa"] = vals
        recs.append(e)
    swe.set_sid_mode(swe.SIDM_FAGAN_BRADLEY, 0.0, 0.0)
    return recs


# ------------------------------------------------------------------ houses
def gen_houses():
    recs = []
    for place, ep in HOUSE_CASES:
        lat, lon = PLACES[place]
        e = epoch(*ep)
        jd_ut = e["jd_ut"]
        rec = {
            "place": place, "lat_deg": lat, "lon_deg": lon,
            **e,
            "sidtime_hours": swe.sidtime(jd_ut),
            "obliquity": ecl_nut(jd_ut),
            "systems": {},
        }
        results = {}
        for h in HSYS:
            hb = h.encode("ascii")
            entry = {"name": swe.house_name(hb)}
            try:
                cusps, ascmc = swe.houses_ex(jd_ut, lat, lon, hb, 0)
            except swe.Error as exc:
                entry["error"] = str(exc)
                entry["undefined_at_latitude"] = True
                results[h] = entry
                continue
            entry.update({
                "cusps_deg": list(cusps),
                "ascendant_deg": ascmc[0],
                "mc_deg": ascmc[1],
                "armc_deg": ascmc[2],
                "vertex_deg": ascmc[3],
                "equatorial_ascendant_deg": ascmc[4],
                "coascendant_koch_deg": ascmc[5],
                "coascendant_munkasey_deg": ascmc[6],
                "polar_ascendant_deg": ascmc[7],
            })
            results[h] = entry
        porph = results["O"]["cusps_deg"]
        for h, entry in results.items():
            if h != "O" and "cusps_deg" in entry:
                entry["fell_back_to_porphyry"] = all(
                    abs(a - b) < 1e-9 for a, b in zip(entry["cusps_deg"], porph))
        rec["systems"] = results
        recs.append(rec)
    return recs


# --------------------------------------------------------------- positions
def gen_positions():
    recs = []
    for ep in POSITION_EPOCHS:
        e = epoch(*ep)
        jd_tt, jd_ut = e["jd_tt"], e["jd_ut"]
        bodies = {}
        for name, ipl in BODIES:
            xx, rf = swe.calc(jd_tt, ipl, POS_FLAGS)
            bodies[name] = {
                "ipl": ipl,
                "lon_deg": xx[0], "lat_deg": xx[1], "dist_au": xx[2],
                "lon_speed_deg_per_day": xx[3],
                "lat_speed_deg_per_day": xx[4],
                "dist_speed_au_per_day": xx[5],
                "retflags": rf,
            }
        e["bodies"] = bodies
        swe.set_sid_mode(swe.SIDM_LAHIRI, 0.0, 0.0)
        _, aya = swe.get_ayanamsa_ex_ut(jd_ut, swe.FLG_MOSEPH)
        sid = {}
        for name, ipl in [("sun", swe.SUN), ("moon", swe.MOON)]:
            xx, rf = swe.calc(jd_tt, ipl, SID_FLAGS)
            sid[name] = {
                "lon_deg": xx[0], "lat_deg": xx[1], "dist_au": xx[2],
                "lon_speed_deg_per_day": xx[3],
                "retflags": rf,
                "ayanamsa_applied_deg": norm360(bodies[name]["lon_deg"] - xx[0]),
            }
        swe.set_sid_mode(swe.SIDM_FAGAN_BRADLEY, 0.0, 0.0)
        e["sidereal_lahiri"] = {
            "sidmode": swe.SIDM_LAHIRI,
            "flags": SID_FLAGS,
            "get_ayanamsa_ex_ut_deg": aya,
            "bodies": sid,
        }
        recs.append(e)
    return recs


# ----------------------------------------------------------------- sunrise
def gen_sunrise():
    recs = []
    for place, (y, m, d) in SUNRISE_CASES:
        lat, lon = PLACES[place]
        jd0 = swe.julday(y, m, d, 0.0, swe.GREG_CAL)
        rec = {
            "place": place, "lat_deg": lat, "lon_deg": lon, "alt_m": 0.0,
            "date_utc": "%04d-%02d-%02d" % (y, m, d),
            "search_start_jd_ut": jd0,
            "events": {},
        }
        for ev, base in [("rise", swe.CALC_RISE), ("set", swe.CALC_SET)]:
            for vname, bits, desc in RISE_VARIANTS:
                rsmi = base | bits
                res, tret = swe.rise_trans(jd0, swe.SUN, rsmi,
                                           (lon, lat, 0.0), 0.0, 0.0,
                                           swe.FLG_MOSEPH)
                t = tret[0]
                yy, mm, dd, hh, mi, ss = swe.jdut1_to_utc(t, swe.GREG_CAL)
                ry, rm, rd, rh = swe.revjul(t, swe.GREG_CAL)
                rec["events"]["%s_%s" % (ev, vname)] = {
                    "rsmi": rsmi,
                    "convention": desc,
                    "result_code": res,
                    "jd_ut": t,
                    "utc": iso(yy, mm, dd, hh, mi, ss),
                    "revjul": [ry, rm, rd, rh],
                }
        recs.append(rec)
    return recs


def main():
    swe.set_ephe_path("")
    swe.set_sid_mode(swe.SIDM_FAGAN_BRADLEY, 0.0, 0.0)
    meta = {
        "generator": "scripts/golden/generate_swisseph.py",
        "versions": {
            "python": platform.python_version(),
            "pyswisseph": swe.__version__,
            "swe_version": swe.version,
        },
        "ephemeris": {
            "ephe_path": "",
            "flags_positions": POS_FLAGS,
            "flags_positions_names": "FLG_MOSEPH|FLG_SPEED",
            "flags_sidereal": SID_FLAGS,
            "flags_sidereal_names": "FLG_MOSEPH|FLG_SPEED|FLG_SIDEREAL",
            "tidal_acceleration": swe.get_tid_acc(),
            "note": "Built-in Moshier ephemeris; no Swiss Ephemeris data "
                    "files (se*.se1) or JPL kernels are used.",
        },
        "conventions": {
            "time": "jd_tt, jd_ut = swe.utc_to_jd(y,m,d,h,mi,s,GREG_CAL). "
                    "jd_ut is UT1 = UTC (no UT1-UTC); jd_tt = jd_ut + "
                    "delta-T with swisseph's leap-second table (37 s carried "
                    "forward after 2017). delta_t_s = deltat_ex(jd_ut, "
                    "FLG_MOSEPH)*86400.",
            "angles": "degrees; longitudes 0..360 east of the equinox; "
                      "speeds in deg/day; distances in AU (AU/day).",
            "ayanamsa": "get_ayanamsa_ut(jd_ut): ayanamsa relative to the "
                        "mean equinox ('without nutation'). "
                        "get_ayanamsa_ex_ut(jd_ut, FLG_MOSEPH): relative "
                        "to the true equinox of date; equals tropical minus "
                        "sidereal longitude when FLG_SIDEREAL is used. "
                        "Both evaluated at jd_ut after set_sid_mode(mode,0,0).",
            "houses": "swe.houses_ex(jd_ut, lat, lon, hsys, 0): tropical. "
                      "ARMC = sidereal time at Greenwich (true equinox, "
                      "with nutation) * 15 + lon; cusps are computed with "
                      "the TRUE obliquity (mean + nutation in obliquity), "
                      "recorded in 'obliquity.true_obliquity_deg' from "
                      "calc_ut(jd_ut, ECL_NUT, FLG_MOSEPH). ascmc order: "
                      "asc, mc, armc, vertex, equatorial asc, co-asc Koch, "
                      "co-asc Munkasey, polar asc. pyswisseph raises "
                      "swisseph.Error where the C library returns retc=-1 "
                      "with Porphyry cusps (Placidus/Koch inside polar "
                      "circles); recorded as 'error'.",
            "positions": "swe.calc(jd_tt, ipl, FLG_MOSEPH|FLG_SPEED): "
                         "geocentric apparent (light-time, gravitational "
                         "deflection, aberration), true ecliptic and "
                         "equinox of date, tropical. "
                         "xx = [lon, lat, dist_au, dlon, dlat, ddist].",
            "sunrise": "swe.rise_trans(jd0, SUN, rsmi, (lon, lat, 0), "
                       "atpress=0 (->1013.25 mbar), attemp=0 (->15 C), "
                       "FLG_MOSEPH); jd0 = 0h UT of the civil date; the "
                       "first event after jd0 is returned. Default rsmi = "
                       "CALC_RISE/CALC_SET: upper limb with refraction. "
                       "|BIT_NO_REFRACTION: upper limb, geometric. "
                       "|BIT_DISC_CENTER: centre with refraction. UTC via "
                       "jdut1_to_utc and revjul (hour as decimal).",
            "kp": "swisseph has no KP sub-lord tables; section left empty.",
        },
    }
    data = {
        "meta": meta,
        "ayanamsa": gen_ayanamsa(),
        "houses": gen_houses(),
        "positions": gen_positions(),
        "kp": [],
        "sunrise": gen_sunrise(),
    }
    data["meta"]["counts"] = {k: len(v) for k, v in data.items() if k != "meta"}
    with open(OUT, "w") as f:
        json.dump(data, f, indent=1, allow_nan=False)
        f.write("\n")
    print("wrote", OUT, data["meta"]["counts"])


if __name__ == "__main__":
    sys.exit(main())
