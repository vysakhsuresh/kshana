#!/usr/bin/env python3
"""Generate golden test vectors for Kshana from reference Python libraries.

Everything runs OFFLINE.  Nothing is downloaded at runtime:
  * ERFA (pyerfa) is a pure C library (SOFA-derived) - no data files.
  * jplephem reads data/de421.bsp directly.
  * Skyfield is pointed at data/de421.bsp through ``skyfield.api.load_file``
    and uses ``load.timescale(builtin=True)`` (bundled leap-second / delta-T
    tables), so it never tries to fetch finals2000A.all or leap files.
  * UT1-UTC comes from data/finals2000A.all, parsed by this script
    (columns 59-68, the IERS "final/prediction" UT1-UTC value in seconds).

Setup (once):

    python3 -m venv /path/to/oracle-venv
    /path/to/oracle-venv/bin/pip install numpy jplephem pyerfa skyfield

Run (from anywhere; paths are resolved relative to this file):

    /path/to/oracle-venv/bin/python scripts/golden/generate.py

Outputs (overwritten each run):

    tests/golden/frames.json     ERFA Earth-attitude quantities (ERA, GMST, GST,
                                 obliquity, nutation, Fukushima-Williams angles,
                                 pnm06a matrix) at 24 TT epochs 1950-2050.
    tests/golden/states.json     Raw per-segment SPK states from jplephem for
                                 de421.bsp at 12 TDB epochs (km, km/s).
    tests/golden/apparent.json   Skyfield apparent geocentric ecliptic-of-date
                                 positions of Sun, Moon and planets at 20 epochs,
                                 plus topocentric Moon from Kochi.

The versions used to produce the committed vectors are recorded in the
"meta" block of each file and in tests/golden/README.md.  Any change to the
library versions should be reflected by re-running this script.

Conventions
-----------
* Two-part Julian dates are given as [jd1, jd2] with jd1 a half-integer
  (the 0h UTC Julian date of the civil day) and jd2 the day fraction (+ the
  UTC->TT or UTC->UT1 offset for those scales), i.e. exactly the split that
  ERFA's dtf2d/utctai/taitt produce.  The epoch is jd1 + jd2.
* TT is the authoritative epoch of every record.  The UTC ISO string is
  the input from which TT was derived with ERFA (utctai + taitt).  Before
  1972 ERFA uses the historic "rubber-second" UTC (1961-1971) and TAI-UTC =
  0 before 1960; Skyfield's builtin table instead assumes 10 s before 1972.
  To stay independent of that, Skyfield Time objects are constructed from
  the ERFA TT two-part JD via ``ts.tt_jd(jd1, jd2)``, never from UTC.
"""

import json
import os
import sys
import warnings
from datetime import date

import numpy as np
import erfa
import erfa.version
import jplephem
from jplephem.spk import SPK
import skyfield
from skyfield.api import load, load_file, wgs84
from skyfield.data import iers as sf_iers

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, os.pardir, os.pardir))
DATA = os.path.join(ROOT, "data")
OUT = os.path.join(ROOT, "tests", "golden")
SPK_PATH = os.path.join(DATA, "de421.bsp")
FINALS_PATH = os.path.join(DATA, "finals2000A.all")

T0 = 2451545.0          # J2000.0 as a JD (TT/TDB)
DAY_S = 86400.0
DJM0 = 2400000.5

# ERFA emits "dubious year" warnings for dates far from the leap-second
# table (before 1960 / after ~5 years past the last known leap second).
# The values are still deterministic (last known TAI-UTC is carried forward).
warnings.simplefilter("ignore", erfa.ErfaWarning)


def versions():
    return {
        "python": sys.version.split()[0],
        "numpy": np.__version__,
        "pyerfa": erfa.__version__,
        "erfa_c_library": erfa.version.erfa_version,
        "sofa_release": erfa.version.sofa_version,
        "jplephem": jplephem.__version__,
        "skyfield": skyfield.__version__,
        "generated_on": date.today().isoformat(),
    }


def iso(y, mo, d, h, mi, s):
    return "%04d-%02d-%02dT%02d:%02d:%02dZ" % (y, mo, d, h, mi, s)


def utc_to_tt(y, mo, d, h, mi, s):
    """Return (utc1, utc2), (tt1, tt2), tai_utc using ERFA."""
    u1, u2 = erfa.dtf2d("UTC", y, mo, d, h, mi, float(s))
    a1, a2 = erfa.utctai(u1, u2)
    t1, t2 = erfa.taitt(a1, a2)
    dat = erfa.dat(y, mo, d, u2)
    return (u1, u2), (t1, t2), float(dat)


# --------------------------------------------------------------------------
# finals2000A.all: UT1-UTC
# --------------------------------------------------------------------------

def parse_finals(path):
    """Parse UT1-UTC from finals2000A.all.

    Columns (1-based, IERS readme.finals2000A):
      7-15   MJD (UTC, at 0h)
      58     IERS flag for UT1-UTC ('I' final, 'P' prediction, ' ' none)
      59-68  UT1-UTC (seconds)
    Rows with an empty UT1-UTC field are skipped.
    """
    mjd, dut1, flag = [], [], []
    with open(path) as f:
        for line in f:
            field = line[58:68].strip()
            if not field:
                continue
            mjd.append(float(line[7:15]))
            dut1.append(float(field))
            flag.append(line[57])
    return np.array(mjd), np.array(dut1), flag


def interp_dut1(mjd_tab, dut1_tab, flags, mjd_utc):
    """Linear interpolation of UT1-UTC between the two bracketing daily rows.

    Returns (value, info) or (None, info) if outside the table.
    """
    if mjd_utc < mjd_tab[0] or mjd_utc > mjd_tab[-1]:
        return None, {"in_range": False, "table_first_mjd": float(mjd_tab[0]),
                      "table_last_mjd": float(mjd_tab[-1])}
    i = int(np.searchsorted(mjd_tab, mjd_utc, side="right")) - 1
    i = max(0, min(i, len(mjd_tab) - 2))
    m0, m1 = mjd_tab[i], mjd_tab[i + 1]
    v0, v1 = dut1_tab[i], dut1_tab[i + 1]
    frac = (mjd_utc - m0) / (m1 - m0)
    val = v0 + frac * (v1 - v0)
    return float(val), {
        "in_range": True,
        "rows": [[float(m0), float(v0), flags[i]], [float(m1), float(v1), flags[i + 1]]],
        "fraction": float(frac),
    }


# --------------------------------------------------------------------------
# frames.json (ERFA)
# --------------------------------------------------------------------------

FRAME_EPOCHS = [  # (y, m, d, H, M, S) UTC -- 24 epochs spread over 1950-2050
    (1950, 1, 1, 0, 0, 0),
    (1954, 3, 15, 6, 30, 0),
    (1958, 7, 4, 18, 45, 10),
    (1963, 11, 22, 19, 30, 0),
    (1967, 5, 9, 3, 0, 0),
    (1971, 9, 17, 23, 59, 59),
    (1976, 2, 29, 12, 0, 0),
    (1980, 8, 8, 8, 8, 8),
    (1984, 4, 1, 15, 20, 30),
    (1989, 10, 31, 21, 0, 0),
    (1993, 6, 21, 4, 12, 0),
    (1997, 12, 25, 9, 45, 0),
    (2000, 1, 1, 12, 0, 0),
    (2006, 3, 29, 10, 11, 12),
    (2010, 7, 15, 0, 0, 1),
    (2015, 1, 20, 2, 2, 2),
    (2019, 9, 23, 7, 50, 0),
    (2024, 4, 8, 18, 17, 0),
    (2028, 11, 11, 11, 11, 11),
    (2032, 2, 14, 5, 5, 5),
    (2037, 8, 1, 13, 0, 0),
    (2041, 5, 5, 22, 30, 0),
    (2045, 10, 10, 1, 1, 1),
    (2050, 12, 31, 23, 0, 0),
]


def make_frames(ts, finals):
    mjd_tab, dut1_tab, flags = finals
    recs = []
    for (y, mo, d, h, mi, s) in FRAME_EPOCHS:
        (u1, u2), (t1, t2), dat = utc_to_tt(y, mo, d, h, mi, s)
        mjd_utc = (u1 - DJM0) + u2
        dut1, info = interp_dut1(mjd_tab, dut1_tab, flags, mjd_utc)
        if dut1 is not None:
            ut1 = (u1, u2 + dut1 / DAY_S)
            delta_t = 32.184 + dat - dut1
            ut1_src = {
                "source": "finals2000A.all columns 59-68 (UT1-UTC, seconds), "
                          "linear interpolation between the two bracketing 0h-UTC rows",
                **info,
            }
        else:
            # Outside the IERS file: fall back to Skyfield's builtin delta-T
            # (iers.npz daily table 1973-2027, Stephenson-Morrison-Hohenkerk
            # 2016 historic table / parabola outside it).  UT1 = TT - delta_T.
            delta_t = float(ts.delta_t_function(t1 + t2))
            ut1 = (t1, t2 - delta_t / DAY_S)
            dut1 = 32.184 + dat - delta_t
            ut1_src = {
                "source": "skyfield %s builtin Timescale.delta_t_function(TT) "
                          "(load.timescale(builtin=True)); UT1 = TT - delta_T; "
                          "finals2000A.all does not cover this date" % skyfield.__version__,
                **info,
            }
        ut1_1, ut1_2 = ut1
        era = erfa.era00(ut1_1, ut1_2)
        gmst = erfa.gmst06(ut1_1, ut1_2, t1, t2)
        gst = erfa.gst06a(ut1_1, ut1_2, t1, t2)
        obl = erfa.obl06(t1, t2)
        dpsi_b, deps_b = erfa.nut00b(t1, t2)
        dpsi_a, deps_a = erfa.nut06a(t1, t2)
        gamb, phib, psib, epsa = erfa.pfw06(t1, t2)
        pnm = erfa.pnm06a(t1, t2)
        recs.append({
            "utc": iso(y, mo, d, h, mi, s),
            "tt_jd": [float(t1), float(t2)],
            "tai_utc_s": dat,
            "ut1_jd": [float(ut1_1), float(ut1_2)],
            "ut1_utc_s": float(dut1),
            "delta_t_s": float(delta_t),
            "ut1_source": ut1_src,
            "era00_rad": float(era),
            "gmst06_rad": float(gmst),
            "gst06a_rad": float(gst),
            "obl06_rad": float(obl),
            "nut00b_rad": {"dpsi": float(dpsi_b), "deps": float(deps_b)},
            "nut06a_rad": {"dpsi": float(dpsi_a), "deps": float(deps_a)},
            "pfw06_rad": {"gamb": float(gamb), "phib": float(phib),
                          "psib": float(psib), "epsa": float(epsa)},
            "pnm06a": [[float(v) for v in row] for row in pnm],
        })
    meta = {
        "description": "ERFA (SOFA-derived) Earth attitude reference values. "
                       "Angles in radians. pnm06a is the 3x3 bias-precession-nutation "
                       "matrix (GCRS -> true equator & equinox of date), rows as returned "
                       "by erfa.pnm06a, i.e. r_true = pnm06a @ r_gcrs.",
        "functions": {
            "tt_jd": "erfa.dtf2d('UTC', ...) -> erfa.utctai -> erfa.taitt (two-part JD, jd1 half-integer)",
            "tai_utc_s": "erfa.dat(y, m, d, fd)",
            "ut1_jd": "[utc1, utc2 + ut1_utc/86400] when finals2000A.all covers the date, else [tt1, tt2 - delta_t/86400]",
            "era00_rad": "erfa.era00(ut1_1, ut1_2)",
            "gmst06_rad": "erfa.gmst06(ut1_1, ut1_2, tt1, tt2)",
            "gst06a_rad": "erfa.gst06a(ut1_1, ut1_2, tt1, tt2)",
            "obl06_rad": "erfa.obl06(tt1, tt2)  (IAU 2006 mean obliquity)",
            "nut00b_rad": "erfa.nut00b(tt1, tt2) -> (dpsi, deps)",
            "nut06a_rad": "erfa.nut06a(tt1, tt2) -> (dpsi, deps)",
            "pfw06_rad": "erfa.pfw06(tt1, tt2) -> (gamb, phib, psib, epsa)",
            "pnm06a": "erfa.pnm06a(tt1, tt2)",
        },
        "finals2000A_coverage_mjd": [float(mjd_tab[0]), float(mjd_tab[-1])],
        "versions": versions(),
        "count": len(recs),
    }
    return {"meta": meta, "records": recs}


# --------------------------------------------------------------------------
# states.json (jplephem, raw SPK segments)
# --------------------------------------------------------------------------

STATE_EPOCHS = [  # (y, m, d, H, M, S) in TDB -- 12 epochs
    (1950, 1, 1, 0, 0, 0),
    (1960, 6, 15, 12, 0, 0),
    (1969, 7, 20, 20, 17, 40),
    (1980, 3, 1, 6, 0, 0),
    (1991, 11, 11, 0, 0, 0),
    (2000, 1, 1, 12, 0, 0),
    (2008, 8, 8, 8, 8, 8),
    (2017, 8, 21, 18, 26, 40),
    (2025, 10, 15, 0, 0, 0),
    (2033, 4, 4, 4, 4, 4),
    (2042, 9, 9, 9, 0, 0),
    (2050, 12, 31, 0, 0, 0),
]

SEGMENTS = [(3, 399), (3, 301), (0, 3), (0, 10)] + [(0, i) for i in range(1, 10)]


def et_seconds(y, mo, d, h, mi, s):
    djm0, djm = erfa.cal2jd(y, mo, d)       # djm0 = 2400000.5, djm = MJD at 0h
    jd0 = djm0 + djm                         # half-integer, exact in double
    # (jd0 - T0) is a half-integer; times 86400 is an exact integer.
    return (jd0 - T0) * DAY_S + (h * 3600 + mi * 60 + s)


def make_states(kernel):
    recs = []
    for (y, mo, d, h, mi, s) in STATE_EPOCHS:
        et = et_seconds(y, mo, d, h, mi, s)
        tdb1, tdb2 = T0, et / DAY_S
        segs = {}
        for (center, target) in SEGMENTS:
            seg = kernel[center, target]
            pos, vel_per_day = seg.compute_and_differentiate(tdb1, tdb2)
            vel = vel_per_day / DAY_S
            segs["%d_rel_%d" % (target, center)] = {
                "center": center, "target": target,
                "position_km": [float(v) for v in pos],
                "velocity_km_s": [float(v) for v in vel],
            }

        def arr(key, what):
            return np.array(segs[key][what])

        derived = {}
        for name, expr, plus, minus in [
            ("earth_rel_ssb", "399_rel_3 + 3_rel_0", ["3_rel_0", "399_rel_3"], []),
            ("moon_rel_ssb", "301_rel_3 + 3_rel_0", ["3_rel_0", "301_rel_3"], []),
            ("moon_rel_earth", "301_rel_3 - 399_rel_3", ["301_rel_3"], ["399_rel_3"]),
            ("sun_rel_earth", "10_rel_0 - (3_rel_0 + 399_rel_3)", ["10_rel_0"], ["3_rel_0", "399_rel_3"]),
        ]:
            p = sum(arr(k, "position_km") for k in plus) - sum((arr(k, "position_km") for k in minus), np.zeros(3))
            v = sum(arr(k, "velocity_km_s") for k in plus) - sum((arr(k, "velocity_km_s") for k in minus), np.zeros(3))
            derived[name] = {"chain": expr,
                             "position_km": [float(x) for x in p],
                             "velocity_km_s": [float(x) for x in v]}
        recs.append({
            "tdb_calendar": iso(y, mo, d, h, mi, s).replace("Z", " TDB"),
            "et_s": float(et),
            "tdb_jd": [tdb1, tdb2],
            "segments": segs,
            "derived": derived,
        })
    meta = {
        "description": "Raw Chebyshev (SPK type 2) states from de421.bsp evaluated with "
                       "jplephem, one entry per segment, no chaining.  Positions in km, "
                       "velocities in km/s, ICRF/J2000 axes, relative to the segment centre.",
        "kernel": os.path.basename(SPK_PATH),
        "kernel_size_bytes": os.path.getsize(SPK_PATH),
        "method": "seg = SPK.open(de421.bsp)[center, target]; "
                  "pos_km, vel_km_per_day = seg.compute_and_differentiate(2451545.0, et/86400.0); "
                  "velocity_km_s = vel_km_per_day / 86400.  jplephem internally forms "
                  "seconds = (tdb - 2451545.0)*86400 + tdb2*86400, so the evaluation "
                  "epoch is exactly et seconds (TDB) past J2000 up to one rounding of et/86400*86400.",
        "et_definition": "TDB seconds past J2000 (JD 2451545.0 TDB).  et = (JD_0h - 2451545.0)*86400 + seconds-of-day; "
                         "both terms are exact integers in double precision.",
        "chaining": {
            "segments": "raw, independent; key '<target>_rel_<center>'",
            "earth_rel_ssb": "segments['3_rel_0'] + segments['399_rel_3']",
            "moon_rel_ssb": "segments['3_rel_0'] + segments['301_rel_3']",
            "moon_rel_earth": "segments['301_rel_3'] - segments['399_rel_3']",
            "sun_rel_earth": "segments['10_rel_0'] - (segments['3_rel_0'] + segments['399_rel_3'])",
        },
        "versions": versions(),
        "count": len(recs),
    }
    return {"meta": meta, "records": recs}


# --------------------------------------------------------------------------
# apparent.json (Skyfield)
# --------------------------------------------------------------------------

APPARENT_EPOCHS = [  # (y, m, d, H, M, S) UTC -- 20 epochs spread over 1950-2050
    (1950, 1, 1, 0, 0, 0),
    (1955, 4, 18, 12, 0, 0),
    (1960, 9, 1, 3, 30, 0),
    (1966, 2, 20, 20, 0, 0),
    (1971, 7, 7, 7, 7, 7),
    (1976, 12, 12, 0, 0, 0),
    (1982, 5, 25, 16, 45, 0),
    (1987, 10, 3, 9, 15, 0),
    (1993, 3, 14, 1, 59, 26),
    (1998, 8, 27, 22, 0, 0),
    (2000, 1, 1, 12, 0, 0),
    (2004, 6, 8, 8, 20, 0),
    (2009, 11, 19, 14, 0, 0),
    (2015, 4, 4, 11, 0, 0),
    (2020, 9, 9, 5, 45, 30),
    (2026, 2, 2, 2, 2, 2),
    (2031, 7, 23, 19, 0, 0),
    (2037, 1, 1, 0, 0, 0),
    (2043, 5, 30, 10, 10, 10),
    (2050, 12, 31, 23, 0, 0),
]

BODIES = [  # (label, skyfield target name, NAIF code)
    ("sun", "sun", 10),
    ("moon", "moon", 301),
    ("mercury", "mercury", 199),
    ("venus", "venus", 299),
    ("mars_barycenter", "mars barycenter", 4),
    ("jupiter_barycenter", "jupiter barycenter", 5),
    ("saturn_barycenter", "saturn barycenter", 6),
    ("uranus_barycenter", "uranus barycenter", 7),
    ("neptune_barycenter", "neptune barycenter", 8),
    ("pluto_barycenter", "pluto barycenter", 9),
]

KOCHI = {"latitude_deg": 9.9312, "longitude_deg": 76.2673, "elevation_m": 0.0}


def make_apparent(ts, eph):
    earth = eph["earth"]
    moon = eph["moon"]
    kochi = wgs84.latlon(KOCHI["latitude_deg"], KOCHI["longitude_deg"],
                         elevation_m=KOCHI["elevation_m"])
    topos = earth + kochi
    recs = []
    for (y, mo, d, h, mi, s) in APPARENT_EPOCHS:
        (u1, u2), (t1, t2), dat = utc_to_tt(y, mo, d, h, mi, s)
        t = ts.tt_jd(t1, t2)
        assert abs((t.whole - t1) + (t.tt_fraction - t2)) < 1e-12
        geo = earth.at(t)
        bodies = {}
        for label, name, code in BODIES:
            app = geo.observe(eph[name]).apparent()
            lat, lon, dist = app.ecliptic_latlon(epoch="date")
            lat0, lon0, dist0 = app.ecliptic_latlon()      # J2000 ecliptic
            ra, dec, _ = app.radec(epoch="date")
            bodies[label] = {
                "naif": code,
                "lon_deg": float(lon.degrees),
                "lat_deg": float(lat.degrees),
                "distance_km": float(dist.km),
                "lon_j2000_deg": float(lon0.degrees),
                "lat_j2000_deg": float(lat0.degrees),
                "ra_date_hours": float(ra.hours),
                "dec_date_deg": float(dec.degrees),
            }
        app_m = topos.at(t).observe(moon).apparent()
        lat, lon, dist = app_m.ecliptic_latlon(epoch="date")
        ut1_whole, ut1_frac = t.whole, t.ut1_fraction
        recs.append({
            "utc": iso(y, mo, d, h, mi, s),
            "tt_jd": [float(t1), float(t2)],
            "tai_utc_s": dat,
            "skyfield_delta_t_s": float(t.delta_t),
            "skyfield_ut1_jd": [float(ut1_whole), float(ut1_frac)],
            "skyfield_tdb_jd": [float(t.whole), float(t.tdb_fraction)],
            "geocentric": bodies,
            "moon_topocentric_kochi": {
                "lon_deg": float(lon.degrees),
                "lat_deg": float(lat.degrees),
                "distance_km": float(dist.km),
            },
        })
    meta = {
        "description": "Skyfield apparent positions (light-time, gravitational deflection, "
                       "aberration; no refraction) expressed in the true ecliptic and "
                       "equinox of date.  lon/lat in degrees (lon 0-360 east of the true "
                       "equinox of date), distance in km.",
        "kernel": os.path.basename(SPK_PATH),
        "timescale": "load.timescale(builtin=True): Skyfield's bundled leap-second and "
                     "delta-T tables (no download).  Time objects built from the ERFA TT "
                     "two-part JD with ts.tt_jd(jd1, jd2); the UTC string is informational.",
        "method": "lat, lon, dist = earth.at(t).observe(body).apparent().ecliptic_latlon(epoch='date'). "
                  "Verified in skyfield/positionlib.py: ecliptic_latlon(epoch=...) with a non-None "
                  "epoch calls _Fake(self, epoch).frame_latlon(framelib.ecliptic_frame); for "
                  "epoch='date' the frame time is the position's own t.  framelib.ecliptic_frame "
                  "is documented 'Reference frame of the true ecliptic and equinox of date' and "
                  "its rotation is build_ecliptic_matrix(t) = rot_x(-(mean_obliquity(t.tdb) + "
                  "d_eps)) @ t.M, where t.M = N @ P @ B (frame bias B, IAU 2006 precession P "
                  "from Capitaine et al. 2003 4-angle formulation evaluated at TDB, nutation N "
                  "from the full IAU 2000A series evaluated at TT) and d_eps is the IAU 2000A "
                  "nutation in obliquity.  With epoch=None the J2000 ecliptic (ECLIPJ2000) is used "
                  "instead; those values are the *_j2000_* fields.  apparent() applies light "
                  "deflection by Sun(10), Jupiter(5) and Saturn(6) (599/699 fall back to the "
                  "barycenters in de421), Earth deflection for a topocentric observer, then "
                  "annual (and diurnal, for topocentric) aberration.",
        "moon_topocentric_kochi": {
            "observer": KOCHI,
            "method": "(earth + wgs84.latlon(9.9312, 76.2673, elevation_m=0.0)).at(t).observe(moon)"
                      ".apparent().ecliptic_latlon(epoch='date').  The observer's GCRS position "
                      "uses Skyfield's builtin delta-T (skyfield_delta_t_s / skyfield_ut1_jd "
                      "recorded per record) and no polar motion.",
        },
        "targets": {label: {"skyfield_name": name, "naif": code} for label, name, code in BODIES},
        "versions": versions(),
        "count": len(recs),
    }
    return {"meta": meta, "records": recs}


# --------------------------------------------------------------------------

def dump(name, obj):
    path = os.path.join(OUT, name)
    with open(path, "w") as f:
        json.dump(obj, f, indent=1)
        f.write("\n")
    print("wrote %s (%d records)" % (path, len(obj["records"])))


def main():
    os.makedirs(OUT, exist_ok=True)
    for p in (SPK_PATH, FINALS_PATH):
        if not os.path.exists(p):
            sys.exit("missing %s (see data/README.md)" % p)

    ts = load.timescale(builtin=True)
    eph = load_file(SPK_PATH)
    kernel = SPK.open(SPK_PATH)
    finals = parse_finals(FINALS_PATH)

    # Sanity: our column parser agrees with Skyfield's regex parser.
    with open(FINALS_PATH, "rb") as f:
        sf_mjd, sf_dut1 = sf_iers.parse_dut1_from_finals_all(f)
    assert len(sf_mjd) == len(finals[0]), (len(sf_mjd), len(finals[0]))
    assert np.array_equal(sf_mjd, finals[0]) and np.array_equal(sf_dut1, finals[1])

    dump("frames.json", make_frames(ts, finals))
    dump("states.json", make_states(kernel))
    dump("apparent.json", make_apparent(ts, eph))
    kernel.close()


if __name__ == "__main__":
    main()
