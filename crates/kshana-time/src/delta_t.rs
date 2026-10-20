//! Earth rotation time. UT1 is measured, not computed, so it carries an uncertainty.
//!
//! Two routes to UT1 exist:
//! * `UT1 = UTC + (UT1 − UTC)` with the daily value from IERS Bulletin A (±0.1 ms for
//!   the measured past, growing for predictions). This is the route Kshana uses in
//!   production; the data file is loaded at runtime and stamped in the provenance.
//! * `UT1 = TT − ΔT` with ΔT from the tables below. This is the route for dates before
//!   1962 and the fallback when no bulletin is loaded.
//!
//! ΔT = TT − UT1 is assembled from three published series, joined continuously:
//!
//! * 1973-02-01 … 2026-04-01, monthly (1st of month, 0h UTC): USNO
//!   `maia.usno.navy.mil/ser7/deltat.data`, the IERS Rapid Service/Prediction Centre
//!   series ΔT = 32.184 s + (TAI − UTC) − (UT1 − UTC).
//! * 1955.5 … 1973.0, half-yearly: USNO `ser7/historic_deltat.data`, the atomic-time era
//!   (TT = TAI + 32.184 s; the values of the *Astronomical Almanac* ΔT table).
//! * −720 … 1955.5: Table S15 v.2020 of Morrison, Stephenson, Hohenkerk & Zawilski,
//!   "Addendum 2020 to 'Measurement of the Earth's rotation: 720 BC to AD 2015'",
//!   Proc. R. Soc. A 478 (2021), doi:10.1098/rspa.2020.0776 (HM Nautical Almanac Office,
//!   `astro.ukho.gov.uk/nao/lvm/Table-S15.2020.txt`): the cubic-spline coefficients of
//!   Stephenson, Morrison & Hohenkerk, Proc. R. Soc. A 472 (2016),
//!   doi:10.1098/rspa.2016.0404, as updated.
//!
//! Files as fetched on 2026-10-20 (SHA-256): `deltat.data` 9f88e535…2383cdf6f (639 rows,
//! 1973-02 to 2026-04), `historic_deltat.data` 9f435141…90b392b70b, `Table-S15.2020.txt`
//! cfcb7dfc…ffd19e5e723.
//!
//! Joins. The S15 spline is a smoothing fit: against the atomic-era values it runs low by
//! 0.22 s at 1950.0, 0.68 s at 1955.5 and 0.65 s at 1965.0, so the measured series is
//! preferred from 1955.5 (TAI begins with the Essen–Parry caesium standard, mid-1955) and
//! the spline is carried onto it by a smoothstep offset over 1950.0–1955.5, which keeps ΔT
//! continuous. The half-yearly 1973.0 point and the first monthly point (1973-02-01)
//! join linearly. Before −720 the long-term parabola of Stephenson et al. 2016
//! (ΔT = −320 + 32.5 u², u = (year − 1825)/100) is used, shifted to meet the spline at
//! −720. After the last monthly value ΔT is extrapolated from the rate over its last
//! twelve months plus the same parabola's tidal curvature (2 × 32.5 / 100² = 0.0065 s/yr²).
//!
//! Refresh: append new rows of `deltat.data` to [`MONTHLY`]; nothing else changes.

use crate::civil::{civil_from_days, days_from_civil};
use crate::instant::{Instant, TwoPartJd};

/// ΔT = TT − UT1 with its one-sigma uncertainty, both in seconds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeltaT {
    /// TT − UT1 in seconds.
    pub seconds: f64,
    /// One-sigma uncertainty in seconds.
    pub sigma: f64,
}

/// UT1 − UTC in seconds with its uncertainty, as published in IERS Bulletin A.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ut1 {
    /// UT1 − UTC in seconds.
    pub ut1_minus_utc: f64,
    /// One-sigma uncertainty in seconds.
    pub sigma: f64,
}

/// First month of [`MONTHLY`]: 1973-02-01 0h UTC.
const MONTHLY_START: (i64, u32) = (1973, 2);

/// Monthly ΔT (s) on the 1st of each month at 0h UTC, from 1973-02-01, verbatim from
/// USNO `ser7/deltat.data` (see the module header).
#[rustfmt::skip]
const MONTHLY: &[f64] = &[
    // 1973-02
    43.4724, 43.5648, 43.6737, 43.7782, 43.8763, 43.9562, 44.0315, 44.1132, 44.1982, 44.2952, 44.3936,
    // 1974-01
    44.4841, 44.5646, 44.6425, 44.7386, 44.8370, 44.9302, 44.9986, 45.0584, 45.1284, 45.2064, 45.2980, 45.3897,
    // 1975-01
    45.4761, 45.5632, 45.6450, 45.7375, 45.8284, 45.9133, 45.9820, 46.0407, 46.1067, 46.1825, 46.2789, 46.3713,
    // 1976-01
    46.4567, 46.5445, 46.6311, 46.7302, 46.8284, 46.9247, 46.9970, 47.0709, 47.1451, 47.2362, 47.3413, 47.4319,
    // 1977-01
    47.5214, 47.6049, 47.6837, 47.7781, 47.8771, 47.9687, 48.0348, 48.0942, 48.1608, 48.2460, 48.3439, 48.4355,
    // 1978-01
    48.5344, 48.6325, 48.7294, 48.8365, 48.9353, 49.0319, 49.1013, 49.1591, 49.2286, 49.3070, 49.4018, 49.4945,
    // 1979-01
    49.5861, 49.6805, 49.7602, 49.8556, 49.9489, 50.0347, 50.1019, 50.1622, 50.2260, 50.2968, 50.3831, 50.4599,
    // 1980-01
    50.5387, 50.6160, 50.6866, 50.7658, 50.8454, 50.9187, 50.9761, 51.0278, 51.0843, 51.1538, 51.2319, 51.3063,
    // 1981-01
    51.3808, 51.4526, 51.5160, 51.5985, 51.6809, 51.7573, 51.8133, 51.8532, 51.9014, 51.9603, 52.0328, 52.0985,
    // 1982-01
    52.1668, 52.2316, 52.2938, 52.3680, 52.4465, 52.5180, 52.5751, 52.6178, 52.6668, 52.7340, 52.8056, 52.8792,
    // 1983-01
    52.9565, 53.0445, 53.1268, 53.2197, 53.3024, 53.3747, 53.4335, 53.4778, 53.5300, 53.5845, 53.6523, 53.7256,
    // 1984-01
    53.7882, 53.8367, 53.8830, 53.9443, 54.0042, 54.0536, 54.0856, 54.1084, 54.1463, 54.1914, 54.2452, 54.2958,
    // 1985-01
    54.3427, 54.3911, 54.4320, 54.4898, 54.5456, 54.5977, 54.6355, 54.6532, 54.6776, 54.7174, 54.7741, 54.8253,
    // 1986-01
    54.8713, 54.9161, 54.9581, 54.9997, 55.0476, 55.0912, 55.1132, 55.1328, 55.1532, 55.1898, 55.2416, 55.2838,
    // 1987-01
    55.3222, 55.3613, 55.4063, 55.4629, 55.5111, 55.5524, 55.5812, 55.6004, 55.6262, 55.6656, 55.7168, 55.7698,
    // 1988-01
    55.8197, 55.8615, 55.9130, 55.9663, 56.0220, 56.0700, 56.0939, 56.1105, 56.1314, 56.1611, 56.2068, 56.2583,
    // 1989-01
    56.3000, 56.3399, 56.3790, 56.4283, 56.4804, 56.5352, 56.5697, 56.5983, 56.6328, 56.6739, 56.7332, 56.7972,
    // 1990-01
    56.8553, 56.9111, 56.9755, 57.0471, 57.1136, 57.1738, 57.2226, 57.2597, 57.3073, 57.3643, 57.4334, 57.5016,
    // 1991-01
    57.5653, 57.6333, 57.6973, 57.7711, 57.8407, 57.9058, 57.9576, 57.9975, 58.0426, 58.1043, 58.1679, 58.2389,
    // 1992-01
    58.3092, 58.3833, 58.4537, 58.5401, 58.6228, 58.6917, 58.7410, 58.7836, 58.8406, 58.8986, 58.9714, 59.0438,
    // 1993-01
    59.1218, 59.2003, 59.2747, 59.3574, 59.4434, 59.5242, 59.5850, 59.6343, 59.6928, 59.7588, 59.8386, 59.9111,
    // 1994-01
    59.9845, 60.0564, 60.1231, 60.2042, 60.2804, 60.3530, 60.4012, 60.4440, 60.4900, 60.5578, 60.6324, 60.7059,
    // 1995-01
    60.7853, 60.8664, 60.9387, 61.0277, 61.1103, 61.1870, 61.2454, 61.2881, 61.3378, 61.4036, 61.4760, 61.5525,
    // 1996-01
    61.6287, 61.6846, 61.7433, 61.8132, 61.8823, 61.9497, 61.9969, 62.0343, 62.0714, 62.1202, 62.1810, 62.2382,
    // 1997-01
    62.2950, 62.3506, 62.3995, 62.4754, 62.5463, 62.6136, 62.6571, 62.6942, 62.7383, 62.7926, 62.8567, 62.9146,
    // 1998-01
    62.9659, 63.0217, 63.0807, 63.1462, 63.2053, 63.2599, 63.2844, 63.2961, 63.3126, 63.3422, 63.3871, 63.4339,
    // 1999-01
    63.4673, 63.4979, 63.5319, 63.5679, 63.6104, 63.6444, 63.6642, 63.6739, 63.6926, 63.7147, 63.7518, 63.7927,
    // 2000-01
    63.8285, 63.8557, 63.8804, 63.9075, 63.9393, 63.9691, 63.9799, 63.9833, 63.9938, 64.0093, 64.0400, 64.0670,
    // 2001-01
    64.0908, 64.1068, 64.1282, 64.1584, 64.1833, 64.2094, 64.2117, 64.2073, 64.2116, 64.2223, 64.2500, 64.2761,
    // 2002-01
    64.2998, 64.3192, 64.3450, 64.3735, 64.3943, 64.4151, 64.4132, 64.4118, 64.4097, 64.4168, 64.4329, 64.4511,
    // 2003-01
    64.4734, 64.4893, 64.5053, 64.5269, 64.5471, 64.5597, 64.5512, 64.5371, 64.5359, 64.5415, 64.5544, 64.5654,
    // 2004-01
    64.5736, 64.5891, 64.6015, 64.6176, 64.6374, 64.6549, 64.6530, 64.6379, 64.6372, 64.6400, 64.6543, 64.6723,
    // 2005-01
    64.6876, 64.7052, 64.7313, 64.7575, 64.7811, 64.8001, 64.7995, 64.7876, 64.7831, 64.7921, 64.8096, 64.8311,
    // 2006-01
    64.8452, 64.8597, 64.8850, 64.9175, 64.9480, 64.9794, 64.9895, 65.0028, 65.0138, 65.0371, 65.0773, 65.1122,
    // 2007-01
    65.1464, 65.1833, 65.2145, 65.2494, 65.2921, 65.3279, 65.3413, 65.3452, 65.3496, 65.3711, 65.3972, 65.4296,
    // 2008-01
    65.4573, 65.4868, 65.5152, 65.5450, 65.5781, 65.6127, 65.6288, 65.6370, 65.6493, 65.6760, 65.7097, 65.7461,
    // 2009-01
    65.7768, 65.8025, 65.8237, 65.8595, 65.8973, 65.9323, 65.9509, 65.9534, 65.9628, 65.9839, 66.0147, 66.0420,
    // 2010-01
    66.0699, 66.0961, 66.1310, 66.1683, 66.2072, 66.2356, 66.2409, 66.2335, 66.2349, 66.2441, 66.2751, 66.3054,
    // 2011-01
    66.3246, 66.3406, 66.3624, 66.3957, 66.4289, 66.4619, 66.4749, 66.4751, 66.4829, 66.5056, 66.5383, 66.5706,
    // 2012-01
    66.6030, 66.6340, 66.6569, 66.6925, 66.7289, 66.7579, 66.7708, 66.7740, 66.7846, 66.8103, 66.8400, 66.8779,
    // 2013-01
    66.9069, 66.9443, 66.9763, 67.0258, 67.0716, 67.1100, 67.1266, 67.1331, 67.1458, 67.1717, 67.2091, 67.2460,
    // 2014-01
    67.2810, 67.3136, 67.3457, 67.3890, 67.4318, 67.4666, 67.4858, 67.4989, 67.5111, 67.5353, 67.5711, 67.6070,
    // 2015-01
    67.6439, 67.6765, 67.7117, 67.7591, 67.8012, 67.8402, 67.8606, 67.8822, 67.9120, 67.9546, 68.0055, 68.0514,
    // 2016-01
    68.1024, 68.1577, 68.2044, 68.2665, 68.3188, 68.3704, 68.3964, 68.4094, 68.4305, 68.4630, 68.5078, 68.5537,
    // 2017-01
    68.5927, 68.6298, 68.6671, 68.7135, 68.7623, 68.8033, 68.8245, 68.8373, 68.8477, 68.8689, 68.9006, 68.9355,
    // 2018-01
    68.9676, 68.9875, 69.0176, 69.0499, 69.0823, 69.1070, 69.1134, 69.1142, 69.1207, 69.1356, 69.1646, 69.1964,
    // 2019-01
    69.2202, 69.2452, 69.2733, 69.3032, 69.3326, 69.3541, 69.3582, 69.3442, 69.3376, 69.3377, 69.3432, 69.3540,
    // 2020-01
    69.3612, 69.3752, 69.3890, 69.4092, 69.4265, 69.4386, 69.4241, 69.3921, 69.3693, 69.3575, 69.3593, 69.3630,
    // 2021-01
    69.3594, 69.3510, 69.3538, 69.3582, 69.3673, 69.3679, 69.3514, 69.3273, 69.3033, 69.2892, 69.2881, 69.2908,
    // 2022-01
    69.2945, 69.2914, 69.2861, 69.2835, 69.2816, 69.2799, 69.2527, 69.2213, 69.1975, 69.1891, 69.1942, 69.2036,
    // 2023-01
    69.2039, 69.1986, 69.1993, 69.2084, 69.2183, 69.2300, 69.2201, 69.1988, 69.1814, 69.1723, 69.1727, 69.1724,
    // 2024-01
    69.1752, 69.1797, 69.1874, 69.1983, 69.2018, 69.2044, 69.1879, 69.1588, 69.1322, 69.1250, 69.1304, 69.1345,
    // 2025-01
    69.1377, 69.1366, 69.1384, 69.1471, 69.1542, 69.1550, 69.1406, 69.1219, 69.0994, 69.0909, 69.0909, 69.1042,
    // 2026-01
    69.1099, 69.1133, 69.1168, 69.1330,
];

/// Half-yearly ΔT (s) of the atomic-time era 1955.5–1973.0 from USNO
/// `ser7/historic_deltat.data` (quoted errors 0.001–0.003 s; module header).
#[rustfmt::skip]
const ATOMIC_ERA: &[(f64, f64)] = &[
    (1955.500, 31.24),
    (1956.000, 31.349),
    (1956.500, 31.516),
    (1957.000, 31.677),
    (1957.500, 31.923),
    (1958.000, 32.166),
    (1958.500, 32.449),
    (1959.000, 32.671),
    (1959.500, 32.919),
    (1960.000, 33.150),
    (1960.500, 33.397),
    (1961.000, 33.584),
    (1961.500, 33.804),
    (1962.000, 33.992),
    (1962.500, 34.240),
    (1963.000, 34.466),
    (1963.500, 34.731),
    (1964.000, 35.030),
    (1964.500, 35.400),
    (1965.000, 35.738),
    (1965.500, 36.147),
    (1966.000, 36.546),
    (1966.500, 36.995),
    (1967.000, 37.429),
    (1967.500, 37.879),
    (1968.000, 38.291),
    (1968.500, 38.753),
    (1969.000, 39.204),
    (1969.500, 39.707),
    (1970.000, 40.182),
    (1970.500, 40.706),
    (1971.000, 41.170),
    (1971.500, 41.686),
    (1972.000, 42.227),
    (1972.500, 42.825),
    (1973.000, 43.373),
];

/// Table S15 v.2020 of Morrison et al. (2021): `(K_i, K_{i+1}, [a0, a1, a2, a3])` with
/// ΔT = a0 + a1 t + a2 t² + a3 t³, t = (Y − K_i)/(K_{i+1} − K_i). Valid −720 … 2019.
#[rustfmt::skip]
const S15: &[(f64, f64, [f64; 4])] = &[
    (-720.0, -100.0, [20371.848, -9999.586, 776.247, 409.160]),
    (-100.0, 400.0, [11557.668, -5822.270, 1303.151, -503.433]),
    (400.0, 1000.0, [6535.116, -5671.519, -298.291, 1085.087]),
    (1000.0, 1150.0, [1650.393, -753.210, 184.811, -25.346]),
    (1150.0, 1300.0, [1056.647, -459.628, 108.771, -24.641]),
    (1300.0, 1500.0, [681.149, -421.345, 61.953, -29.414]),
    (1500.0, 1600.0, [292.343, -192.841, -6.572, 16.197]),
    (1600.0, 1650.0, [109.127, -78.697, 10.505, 3.018]),
    (1650.0, 1720.0, [43.952, -68.089, 38.333, -2.127]),
    (1720.0, 1800.0, [12.068, 2.507, 41.731, -37.939]),
    (1800.0, 1810.0, [18.367, -3.481, -1.126, 1.918]),
    (1810.0, 1820.0, [15.678, 0.021, 4.629, -3.812]),
    (1820.0, 1830.0, [16.516, -2.157, -6.806, 3.250]),
    (1830.0, 1840.0, [10.804, -6.018, 2.944, -0.096]),
    (1840.0, 1850.0, [7.634, -0.416, 2.658, -0.539]),
    (1850.0, 1855.0, [9.338, 1.642, 0.261, -0.883]),
    (1855.0, 1860.0, [10.357, -0.486, -2.389, 1.558]),
    (1860.0, 1865.0, [9.040, -0.591, 2.284, -2.477]),
    (1865.0, 1870.0, [8.255, -3.456, -5.148, 2.720]),
    (1870.0, 1875.0, [2.371, -5.593, 3.011, -0.914]),
    (1875.0, 1880.0, [-1.126, -2.314, 0.269, -0.039]),
    (1880.0, 1885.0, [-3.210, -1.893, 0.152, 0.563]),
    (1885.0, 1890.0, [-4.388, 0.101, 1.842, -1.438]),
    (1890.0, 1895.0, [-3.884, -0.531, -2.474, 1.871]),
    (1895.0, 1900.0, [-5.017, 0.134, 3.138, -0.232]),
    (1900.0, 1905.0, [-1.977, 5.715, 2.443, -1.257]),
    (1905.0, 1910.0, [4.923, 6.828, -1.329, 0.720]),
    (1910.0, 1915.0, [11.142, 6.330, 0.831, -0.825]),
    (1915.0, 1920.0, [17.479, 5.518, -1.643, 0.262]),
    (1920.0, 1925.0, [21.617, 3.020, -0.856, 0.008]),
    (1925.0, 1930.0, [23.789, 1.333, -0.831, 0.127]),
    (1930.0, 1935.0, [24.418, 0.052, -0.449, 0.142]),
    (1935.0, 1940.0, [24.164, -0.419, -0.022, 0.702]),
    (1940.0, 1945.0, [24.426, 1.645, 2.086, -1.106]),
    (1945.0, 1950.0, [27.050, 2.499, -1.232, 0.614]),
    (1950.0, 1953.0, [28.932, 1.127, 0.220, -0.277]),
    (1953.0, 1956.0, [30.002, 0.737, -0.610, 0.631]),
    (1956.0, 1959.0, [30.760, 1.409, 1.282, -0.799]),
    (1959.0, 1962.0, [32.652, 1.577, -1.115, 0.507]),
    (1962.0, 1965.0, [33.621, 0.868, 0.406, 0.199]),
    (1965.0, 1968.0, [35.093, 2.275, 1.002, -0.414]),
    (1968.0, 1971.0, [37.956, 3.035, -0.242, 0.202]),
    (1971.0, 1974.0, [40.951, 3.157, 0.364, -0.229]),
    (1974.0, 1977.0, [44.244, 3.199, -0.323, 0.172]),
    (1977.0, 1980.0, [47.291, 3.069, 0.193, -0.192]),
    (1980.0, 1983.0, [50.361, 2.878, -0.384, 0.081]),
    (1983.0, 1986.0, [52.936, 2.354, -0.140, -0.165]),
    (1986.0, 1989.0, [54.984, 1.577, -0.637, 0.448]),
    (1989.0, 1992.0, [56.373, 1.648, 0.708, -0.276]),
    (1992.0, 1995.0, [58.453, 2.235, -0.121, 0.110]),
    (1995.0, 1998.0, [60.678, 2.324, 0.210, -0.313]),
    (1998.0, 2001.0, [62.898, 1.804, -0.729, 0.109]),
    (2001.0, 2004.0, [64.083, 0.674, -0.402, 0.199]),
    (2004.0, 2007.0, [64.553, 0.466, 0.194, -0.017]),
    (2007.0, 2010.0, [65.197, 0.804, 0.144, -0.084]),
    (2010.0, 2013.0, [66.061, 0.839, -0.109, 0.128]),
    (2013.0, 2016.0, [66.920, 1.007, 0.277, -0.095]),
    (2016.0, 2019.0, [68.109, 1.277, -0.007, -0.139]),
];

/// Start of the measured atomic-era series, and the start of the spline-to-series blend.
const ATOMIC_FROM: f64 = 1955.5;
const BLEND_FROM: f64 = 1950.0;

fn s15(y: f64) -> f64 {
    let row = S15
        .iter()
        .find(|r| y >= r.0 && y < r.1)
        .unwrap_or(if y < S15[0].0 {
            &S15[0]
        } else {
            &S15[S15.len() - 1]
        });
    let t = (y - row.0) / (row.1 - row.0);
    let a = row.2;
    a[0] + t * (a[1] + t * (a[2] + t * a[3]))
}

/// Long-term parabola of Stephenson, Morrison & Hohenkerk (2016).
fn parabola(y: f64) -> f64 {
    let u = (y - 1825.0) / 100.0;
    -320.0 + 32.5 * u * u
}

/// Linear interpolation in a `(year, value)` series sorted by year.
fn interp(series: &[(f64, f64)], y: f64) -> f64 {
    let i = series
        .windows(2)
        .position(|w| y <= w[1].0)
        .unwrap_or(series.len() - 2);
    let (x0, v0) = series[i];
    let (x1, v1) = series[i + 1];
    v0 + (v1 - v0) * (y - x0) / (x1 - x0)
}

/// Days in the Gregorian year `y`.
fn year_len(y: i64) -> f64 {
    (days_from_civil(y + 1, 1, 1) - days_from_civil(y, 1, 1)) as f64
}

/// Decimal year of a day number (days since 1970-01-01, fractional).
fn year_of_day(day: f64) -> f64 {
    let (y, _, _) = civil_from_days(day.floor() as i64);
    y as f64 + (day - days_from_civil(y, 1, 1) as f64) / year_len(y)
}

/// Day number (days since 1970-01-01, fractional) of a decimal year.
fn day_of_year(year: f64) -> f64 {
    let y = year.floor() as i64;
    days_from_civil(y, 1, 1) as f64 + (year - y as f64) * year_len(y)
}

/// Months since year 0 of `(year, month)`.
fn month_index(y: i64, m: u32) -> i64 {
    y * 12 + m as i64 - 1
}

/// Day number of the 1st of the `k`-th month of [`MONTHLY`].
fn monthly_day(k: usize) -> i64 {
    let m0 = month_index(MONTHLY_START.0, MONTHLY_START.1) + k as i64;
    days_from_civil(m0.div_euclid(12), m0.rem_euclid(12) as u32 + 1, 1)
}

fn monthly_first_year() -> f64 {
    year_of_day(monthly_day(0) as f64)
}

fn monthly_last_year() -> f64 {
    year_of_day(monthly_day(MONTHLY.len() - 1) as f64)
}

/// ΔT inside the monthly table, linear between the two bracketing month starts.
fn monthly(year: f64) -> f64 {
    let day = day_of_year(year);
    let (y, m, _) = civil_from_days(day.floor() as i64);
    let k = month_index(y, m) - month_index(MONTHLY_START.0, MONTHLY_START.1);
    let k = (k.max(0) as usize).min(MONTHLY.len() - 2);
    let (d0, d1) = (monthly_day(k) as f64, monthly_day(k + 1) as f64);
    MONTHLY[k] + (MONTHLY[k + 1] - MONTHLY[k]) * (day - d0) / (d1 - d0)
}

/// Uncertainty of the historical spline: Morrison & Stephenson (2004), J. Hist. Astron.
/// 35, 327, give σ ≈ 0.8 t² s with t in centuries from 1820; floored by era to the
/// errors USNO quotes for the telescopic record (`historic_deltat.data`: 10–15 s near
/// 1660, about a second in the 1800s) and, after 1900, to the spline's own 0.2–0.7 s
/// disagreement with the atomic-era values.
fn spline_sigma(y: f64) -> f64 {
    let t = (y - 1820.0) / 100.0;
    let floor = if y >= 1900.0 {
        0.5
    } else if y >= 1800.0 {
        1.0
    } else if y >= 1700.0 {
        5.0
    } else if y >= 1600.0 {
        15.0
    } else {
        0.0
    };
    (0.8 * t * t).max(floor)
}

/// ΔT for a decimal year (the Gregorian calendar year plus the elapsed fraction of it).
pub fn delta_t_for_year(year: f64) -> DeltaT {
    let first = monthly_first_year();
    let last = monthly_last_year();
    if year >= first && year <= last {
        // Monthly samples of a smooth series: linear interpolation errs by a few ms.
        return DeltaT {
            seconds: monthly(year),
            sigma: 0.005,
        };
    }
    if year > last {
        // Extrapolation: the rate over the last twelve months plus the long-term tidal
        // curvature. Sigma follows the length-of-day trend's own uncertainty, about
        // 1.2 ms per day of extrapolation: 4 s at ten years, 22 s at fifty.
        let n = MONTHLY.len();
        let rate = MONTHLY[n - 1] - MONTHLY[n - 13];
        let dy = year - last;
        return DeltaT {
            seconds: MONTHLY[n - 1] + rate * dy + 0.5 * 0.0065 * dy * dy,
            sigma: 0.005 + 1.2e-3 * 365.25 * dy,
        };
    }
    let (atomic_last_year, atomic_last) = ATOMIC_ERA[ATOMIC_ERA.len() - 1];
    if year >= atomic_last_year {
        // 1973.0 → 1973-02-01: the half-yearly series meets the monthly one.
        let f = (year - atomic_last_year) / (first - atomic_last_year);
        return DeltaT {
            seconds: atomic_last + (MONTHLY[0] - atomic_last) * f,
            sigma: 0.01,
        };
    }
    if year >= ATOMIC_FROM {
        return DeltaT {
            seconds: interp(ATOMIC_ERA, year),
            sigma: 0.01,
        };
    }
    if year >= BLEND_FROM {
        let offset = ATOMIC_ERA[0].1 - s15(ATOMIC_FROM);
        let s = (year - BLEND_FROM) / (ATOMIC_FROM - BLEND_FROM);
        let w = s * s * (3.0 - 2.0 * s);
        return DeltaT {
            seconds: s15(year) + w * offset,
            sigma: spline_sigma(year).max(offset.abs()),
        };
    }
    if year >= S15[0].0 {
        return DeltaT {
            seconds: s15(year),
            sigma: spline_sigma(year),
        };
    }
    DeltaT {
        seconds: parabola(year) - parabola(S15[0].0) + s15(S15[0].0),
        sigma: spline_sigma(year),
    }
}

/// UT1 as a two-part Julian Date from an instant and a ΔT value.
pub fn ut1_jd_from_delta_t(i: Instant, dt: DeltaT) -> TwoPartJd {
    let tt = i.tt_jd();
    TwoPartJd {
        jd1: tt.jd1,
        jd2: tt.jd2 - dt.seconds / 86_400.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(y: i64, m: u32, d: u32) -> f64 {
        year_of_day(days_from_civil(y, m, d) as f64)
    }

    #[test]
    fn tables_are_sorted_and_contiguous() {
        assert_eq!(MONTHLY.len(), 639);
        assert_eq!(monthly_day(MONTHLY.len() - 1), days_from_civil(2026, 4, 1));
        for w in ATOMIC_ERA.windows(2) {
            assert!((w[1].0 - w[0].0 - 0.5).abs() < 1e-9);
        }
        for w in S15.windows(2) {
            assert_eq!(w[0].1, w[1].0);
        }
        assert_eq!((S15[0].0, S15[S15.len() - 1].1), (-720.0, 2019.0));
    }

    #[test]
    fn pins_published_values() {
        // USNO deltat.data, 1st of month 0h UTC.
        for (y, m, want) in [
            (1990, 1, 56.8553),
            (2000, 1, 63.8285),
            (2010, 1, 66.0699),
            (2020, 1, 69.3612),
            (2026, 4, 69.1330),
        ] {
            let got = delta_t_for_year(at(y, m, 1)).seconds;
            assert!((got - want).abs() < 1e-9, "{y}-{m}: {got} vs {want}");
        }
        // USNO historic_deltat.data (atomic era).
        for (y, want) in [(1960.0, 33.150), (1965.0, 35.738), (1970.0, 40.182)] {
            let got = delta_t_for_year(y).seconds;
            assert!((got - want).abs() < 1e-9, "{y}: {got} vs {want}");
        }
        // Table S15 v.2020: a0 of the rows starting at 1900, 1820 and 1000.
        for (y, want) in [(1900.0, -1.977), (1820.0, 16.516), (1000.0, 1650.393)] {
            let got = delta_t_for_year(y).seconds;
            assert!((got - want).abs() < 1e-9, "{y}: {got} vs {want}");
        }
        // 1950.0 opens the blend with weight 0: the spline value itself.
        assert!((delta_t_for_year(1950.0).seconds - 28.932).abs() < 1e-9);
    }

    #[test]
    fn s15_rows_join_at_the_knots() {
        // The published coefficients meet at the knots to the table's rounding.
        for w in S15.windows(2) {
            let a = w[0].2;
            let end = a[0] + a[1] + a[2] + a[3];
            assert!(
                (end - w[1].2[0]).abs() < 0.01,
                "{}: {end} vs {}",
                w[0].1,
                w[1].2[0]
            );
        }
    }

    #[test]
    fn every_join_is_continuous() {
        let eps = 1e-7;
        for y in [
            -720.0,
            BLEND_FROM,
            ATOMIC_FROM,
            1973.0,
            monthly_first_year(),
            monthly_last_year(),
        ] {
            let a = delta_t_for_year(y - eps).seconds;
            let b = delta_t_for_year(y + eps).seconds;
            // 1950.0 is also an S15 knot, where the rows meet only to their 0.001 s rounding.
            assert!((a - b).abs() < 2e-3, "{y}: {a} vs {b}");
        }
    }

    #[test]
    fn extrapolates_sanely() {
        let d = delta_t_for_year(2026.8);
        assert!(d.seconds > 69.0 && d.seconds < 69.5, "{d:?}");
        assert!(d.sigma > 0.1, "{d:?}");
        // 2100: some tens of seconds above today, with a sigma to match.
        let d = delta_t_for_year(2100.0);
        assert!(
            d.seconds > 69.0 && d.seconds < 120.0 && d.sigma > 20.0,
            "{d:?}"
        );
        // Ancient dates follow the parabola's shape.
        let d = delta_t_for_year(-1000.0);
        assert!(d.seconds > 20_371.0 && d.sigma > 500.0, "{d:?}");
    }
}
