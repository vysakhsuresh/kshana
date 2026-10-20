//! The conditional nakshatra dashas of BPHS ch. 46: Dwadasottari, Shodasottari,
//! Panchottari, Shatabdika, Chaturaseeti-sama, Dwisaptati-sama and Shashtihayani.
//!
//! # Sources
//!
//! The text is BPHS ch. 46 ("Dasha-adhyaya") in the verse numbering of the Sanskrit
//! edition on enjoylearningsanskrit.com (`/scriptures/parashara/chapter-46/verse-N`,
//! read verse by verse for this module; R. Santhanam's translation numbers the same
//! verses identically for this block). The JHora lineage used as the cross-check is
//! PyJHora `src/jhora/horoscope/dhasa/graha/{dwadasottari, shodasottari, panchottari,
//! sataatbika, chathuraaseethi_sama, dwisatpathi, shastihayani}.py` and
//! `applicability.py`. Each system below agrees with PyJHora on lords, years, the
//! counting origin and the applicability test unless a note says otherwise.
//!
//! | System | Applies when (verse) | Counted (verse) | Lords and years (verse) | Cycle |
//! |---|---|---|---|---|
//! | Shodasottari | lagna in the Moon's hora in the dark fortnight, or in the Sun's hora in the bright one (46.24) | Pushya → janma, remainder by 8 (46.25) | Sun 11, Mars 12, Jupiter 13, Saturn 14, Ketu 15, Moon 16, Mercury 17, Venus 18 (46.25–26) | 116 |
//! | Dwadasottari | born in a navamsa of Venus — the lagna in Taurus or Libra navamsa (46.27) | janma → Revati, remainder by 8 (46.27) | Sun 7, Jupiter 9, Ketu 11, Mercury 13, Rahu 15, Mars 17, Saturn 19, Moon 21 (46.28) | 112 |
//! | Panchottari | Cancer lagna in a Cancer dwadasamsa (46.29) | Anuradha (Mitra's star) → janma, remainder by 7 (46.29–30) | Sun 12, Mercury 13, Saturn 14, Mars 15, Venus 16, Moon 17, Jupiter 18 (46.30–31) | 105 |
//! | Shatabdika | vargottama lagna (46.32) | Revati (Pushan's star) → janma, remainder by 7 (46.32–33) | Sun 5, Moon 5, Venus 10, Mercury 10, Jupiter 20, Mars 20, Saturn 30 (46.33–34) | 100 |
//! | Chaturaseeti-sama | lord of the 10th in the 10th (46.35) | Swati (Pavana's star) → janma, remainder by 7 (46.35) | Sun … Saturn, 12 each (46.36) | 84 |
//! | Dwisaptati-sama | lagna lord in the 7th, or the 7th lord in the lagna (46.38–39) | Mula → janma, remainder by 8 (46.37) | Sun … Saturn and Rahu, 9 each (46.37–38) | 72 |
//! | Shashtihayani | the Sun in the lagna sign (46.40) | from Ashwini in groups of 3 and 4 (46.40) | Jupiter, Sun, Mars 10 each; Moon, Mercury, Venus, Saturn, Rahu 6 each (46.41) | 60 |
//!
//! "Remainder by n" counts the nakshatras inclusively from the origin; remainder 1 is the
//! first lord and 0 the last, so the lord index is `(count − 1) mod n`.
//!
//! # Conventions where the text is silent or ambiguous
//!
//! * **Panchottari** (46.29 "arkāṃśa karkalagne", a pada missing a syllable): read, as
//!   JHora does, as "the lagna in the Cancer dwadasamsa"; [`panchottari_applies`] tests the
//!   D12 sign of the lagna only. The stricter reading (rasi lagna also Cancer, i.e. the
//!   first 2°30′ of Cancer) is the subset `sign(lagna) == Cancer && panchottari_applies`.
//! * **Dwadasottari** (46.27 "śukrāṃśake", born in Venus's amsha): read as the lagna's
//!   navamsa sign owned by Venus, as JHora does.
//! * **Shashtihayani groups** (46.40 "dāsrāt trayaṃ catuṣkaṃ ca trayaṃ ceti punaḥ punaḥ"):
//!   the JHora-lineage table alternates 3 and 4 from Ashwini over 27 nakshatras, which
//!   leaves Rahu three (P.Bhadrapada, U.Bhadrapada, Revati): Jupiter Ashwini–Krittika,
//!   Sun Rohini–Punarvasu, Mars Pushya–Magha, Moon P.Phalguni–Chitra, Mercury
//!   Swati–Anuradha, Venus Jyeshtha–U.Ashadha, Saturn Shravana–Shatabhisha. Two other
//!   readings exist and are not implemented: a 28-nakshatra count with Abhijit (Rahu then
//!   gets four), and the literal "3, 4, 3, repeated" pattern (3,4,3,3,4,3,3,4 = 27).
//! * **Balance at birth**: 46.39 says the balance is found "as in Vimshottari", i.e. from
//!   the fraction of the janma nakshatra still to run, times the lord's years. That rule
//!   is used for every system here, Shashtihayani included (as PyJHora does), even though
//!   its lords hold groups of nakshatras.
//! * **Antardashas**: the chapter gives no rule for these systems. Both readings in use
//!   are offered by [`AntarRule`]: `Proportional` (the Vimshottari pattern, each lord's
//!   share `maha × years / cycle`) and `Equal` (each of the n lords one n-th, which is
//!   what PyJHora computes). Either way they start from the mahadasha lord and run in
//!   cycle order. For the two sama systems both rules coincide.
//! * The Shattrimsa-sama dasha of 46.42–43 is outside this module's scope.

use kshana_time::Instant;

use super::{fraction_ns, graha_from_name, graha_name, lay_out_cycle, subdivide_weighted, Period};
use crate::{nakshatra, sign_lord, varga, DashaLord};

/// The seven conditional systems of BPHS ch. 46 implemented here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Conditional {
    /// 112 years, from janma to Revati (46.27–28).
    Dwadasottari,
    /// 116 years, from Pushya (46.24–26).
    Shodasottari,
    /// 105 years, from Anuradha (46.29–31).
    Panchottari,
    /// 100 years, from Revati (46.32–34).
    Shatabdika,
    /// 84 years, from Swati (46.35–36).
    ChaturaseetiSama,
    /// 72 years, from Mula (46.37–39).
    DwisaptatiSama,
    /// 60 years, groups from Ashwini (46.40–41).
    Shashtihayani,
}

impl Conditional {
    /// All seven, in BPHS order.
    pub const ALL: [Conditional; 7] = [
        Conditional::Shodasottari,
        Conditional::Dwadasottari,
        Conditional::Panchottari,
        Conditional::Shatabdika,
        Conditional::ChaturaseetiSama,
        Conditional::DwisaptatiSama,
        Conditional::Shashtihayani,
    ];

    /// English name.
    pub const fn name(self) -> &'static str {
        match self {
            Conditional::Dwadasottari => "Dwadasottari",
            Conditional::Shodasottari => "Shodasottari",
            Conditional::Panchottari => "Panchottari",
            Conditional::Shatabdika => "Shatabdika",
            Conditional::ChaturaseetiSama => "Chaturaseeti-sama",
            Conditional::DwisaptatiSama => "Dwisaptati-sama",
            Conditional::Shashtihayani => "Shashtihayani",
        }
    }

    /// The lords in cycle order.
    pub const fn lords(self) -> &'static [DashaLord] {
        use DashaLord::*;
        match self {
            Conditional::Dwadasottari => &[Sun, Jupiter, Ketu, Mercury, Rahu, Mars, Saturn, Moon],
            Conditional::Shodasottari => &[Sun, Mars, Jupiter, Saturn, Ketu, Moon, Mercury, Venus],
            Conditional::Panchottari => &[Sun, Mercury, Saturn, Mars, Venus, Moon, Jupiter],
            Conditional::Shatabdika => &[Sun, Moon, Venus, Mercury, Jupiter, Mars, Saturn],
            Conditional::ChaturaseetiSama => &[Sun, Moon, Mars, Mercury, Jupiter, Venus, Saturn],
            Conditional::DwisaptatiSama => {
                &[Sun, Moon, Mars, Mercury, Jupiter, Venus, Saturn, Rahu]
            }
            Conditional::Shashtihayani => &[Jupiter, Sun, Mars, Moon, Mercury, Venus, Saturn, Rahu],
        }
    }

    /// Years of each lord, parallel to [`Conditional::lords`].
    pub const fn years(self) -> &'static [u32] {
        match self {
            Conditional::Dwadasottari => &[7, 9, 11, 13, 15, 17, 19, 21],
            Conditional::Shodasottari => &[11, 12, 13, 14, 15, 16, 17, 18],
            Conditional::Panchottari => &[12, 13, 14, 15, 16, 17, 18],
            Conditional::Shatabdika => &[5, 5, 10, 10, 20, 20, 30],
            Conditional::ChaturaseetiSama => &[12; 7],
            Conditional::DwisaptatiSama => &[9; 8],
            Conditional::Shashtihayani => &[10, 10, 10, 6, 6, 6, 6, 6],
        }
    }

    /// Total years of one cycle.
    pub fn cycle_years(self) -> u32 {
        self.years().iter().sum()
    }

    /// Index into [`Conditional::lords`] of the lord ruling a 0-based nakshatra
    /// (0 = Ashwini).
    pub fn lord_index(self, nakshatra_index: usize) -> usize {
        let nak = nakshatra_index % 27;
        let n = self.lords().len();
        // Inclusive count from the origin to janma, minus one.
        let from = |origin: usize| (nak + 27 - origin) % 27 % n;
        match self {
            Conditional::Shodasottari => from(7),      // Pushya
            Conditional::Panchottari => from(16),      // Anuradha
            Conditional::Shatabdika => from(26),       // Revati
            Conditional::ChaturaseetiSama => from(14), // Swati
            Conditional::DwisaptatiSama => from(18),   // Mula
            // Janma → Revati: inclusive count minus one.
            Conditional::Dwadasottari => (26 + 27 - nak) % 27 % n,
            Conditional::Shashtihayani => {
                SHASHTIHAYANI_GROUPS
                    .iter()
                    .position(|&(start, count)| (start..start + count).contains(&nak))
                    .unwrap_or(n - 1)
            }
        }
    }
}

/// Shashtihayani nakshatra groups `(first index, count)` in lord order (JHora lineage;
/// see the module notes).
pub const SHASHTIHAYANI_GROUPS: [(usize, usize); 8] = [
    (0, 3),  // Jupiter: Ashwini, Bharani, Krittika
    (3, 4),  // Sun: Rohini … Punarvasu
    (7, 3),  // Mars: Pushya, Ashlesha, Magha
    (10, 4), // Moon: P.Phalguni … Chitra
    (14, 3), // Mercury: Swati, Vishakha, Anuradha
    (17, 4), // Venus: Jyeshtha … U.Ashadha
    (21, 3), // Saturn: Shravana, Dhanishta, Shatabhisha
    (24, 3), // Rahu: P.Bhadrapada, U.Bhadrapada, Revati
];

/// How antardashas are proportioned (see the module notes).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AntarRule {
    /// Each lord's share is its years over the cycle (the Vimshottari pattern).
    Proportional,
    /// Each lord gets an equal share (PyJHora).
    Equal,
}

/// Mahadashas of `system` from the Moon's nakshatra, the first cut to its balance at
/// birth, repeating the cycle until [`super::HORIZON_YEARS`] after birth. Computed
/// whether or not the system [applies](applies); check that separately.
pub fn conditional(
    system: Conditional,
    moon_sidereal: f64,
    birth: Instant,
    year_ns: i128,
) -> Vec<Period> {
    let nak = nakshatra(moon_sidereal);
    let first = system.lord_index(nak.index);
    let years = system.years();
    let elapsed_ns = fraction_ns(nak.fraction, years[first] as i128 * year_ns);
    let cycle: Vec<(&str, u32)> = system
        .lords()
        .iter()
        .zip(years)
        .map(|(l, y)| (graha_name(*l), *y))
        .collect();
    lay_out_cycle(&cycle, first, elapsed_ns, birth, year_ns, 1)
}

/// Antardashas of a period of `system`: every lord in cycle order from the period's own,
/// proportioned by `rule`. Empty when the label is not one of the system's lords.
pub fn conditional_sub(system: Conditional, period: &Period, rule: AntarRule) -> Vec<Period> {
    let lords = system.lords();
    let Some(first) = graha_from_name(&period.label).and_then(|g| lords.iter().position(|l| *l == g))
    else {
        return Vec::new();
    };
    let n = lords.len();
    let parts: Vec<(&str, u32)> = (0..n)
        .map(|k| (first + k) % n)
        .map(|i| {
            let w = match rule {
                AntarRule::Proportional => system.years()[i],
                AntarRule::Equal => 1,
            };
            (graha_name(lords[i]), w)
        })
        .collect();
    subdivide_weighted(period, &parts)
}

/// What the applicability tests read. Longitudes are sidereal degrees; `grahas` are in
/// the order Sun, Moon, Mars, Mercury, Jupiter, Venus, Saturn, Rahu, Ketu.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChartFacts {
    /// Sidereal ascendant.
    pub lagna: f64,
    /// The nine grahas.
    pub grahas: [f64; 9],
}

fn sign(lon: f64) -> usize {
    super::sign_of(lon)
}

fn graha_index(g: DashaLord) -> usize {
    match g {
        DashaLord::Sun => 0,
        DashaLord::Moon => 1,
        DashaLord::Mars => 2,
        DashaLord::Mercury => 3,
        DashaLord::Jupiter => 4,
        DashaLord::Venus => 5,
        DashaLord::Saturn => 6,
        DashaLord::Rahu => 7,
        DashaLord::Ketu => 8,
    }
}

/// True in the bright fortnight: the Moon less than 180° ahead of the Sun.
pub fn is_shukla(sun_sidereal: f64, moon_sidereal: f64) -> bool {
    (moon_sidereal - sun_sidereal).rem_euclid(360.0) < 180.0
}

/// BPHS 46.27: the lagna's navamsa is Taurus or Libra (owned by Venus).
pub fn dwadasottari_applies(lagna_sidereal: f64) -> bool {
    sign_lord(sign(varga(lagna_sidereal, 9))) == DashaLord::Venus
}

/// BPHS 46.24: the lagna in the Moon's hora (Cancer) in the dark fortnight, or in the
/// Sun's hora (Leo) in the bright fortnight. Parashari hora (D2).
pub fn shodasottari_applies(lagna_sidereal: f64, sun_sidereal: f64, moon_sidereal: f64) -> bool {
    let hora = sign(varga(lagna_sidereal, 2));
    let shukla = is_shukla(sun_sidereal, moon_sidereal);
    (hora == 3 && !shukla) || (hora == 4 && shukla)
}

/// BPHS 46.29 as JHora reads it: the lagna's dwadasamsa (D12) is Cancer.
pub fn panchottari_applies(lagna_sidereal: f64) -> bool {
    sign(varga(lagna_sidereal, 12)) == 3
}

/// BPHS 46.32: the lagna is vargottama (same sign in rasi and navamsa).
pub fn shatabdika_applies(lagna_sidereal: f64) -> bool {
    sign(varga(lagna_sidereal, 9)) == sign(lagna_sidereal)
}

/// BPHS 46.35: the lord of the 10th sign from the lagna occupies it.
pub fn chaturaseeti_sama_applies(facts: &ChartFacts) -> bool {
    let tenth = (sign(facts.lagna) + 9) % 12;
    sign(facts.grahas[graha_index(sign_lord(tenth))]) == tenth
}

/// BPHS 46.38–39: the lagna lord in the 7th sign, or the 7th lord in the lagna sign.
pub fn dwisaptati_sama_applies(facts: &ChartFacts) -> bool {
    let lagna = sign(facts.lagna);
    let seventh = (lagna + 6) % 12;
    let at = |s: usize| sign(facts.grahas[graha_index(sign_lord(s))]);
    at(lagna) == seventh || at(seventh) == lagna
}

/// BPHS 46.40: the Sun in the lagna sign.
pub fn shashtihayani_applies(lagna_sidereal: f64, sun_sidereal: f64) -> bool {
    sign(lagna_sidereal) == sign(sun_sidereal)
}

/// Whether `system` applies to the chart (BPHS ch. 46).
pub fn applies(system: Conditional, facts: &ChartFacts) -> bool {
    let (sun, moon) = (facts.grahas[0], facts.grahas[1]);
    match system {
        Conditional::Dwadasottari => dwadasottari_applies(facts.lagna),
        Conditional::Shodasottari => shodasottari_applies(facts.lagna, sun, moon),
        Conditional::Panchottari => panchottari_applies(facts.lagna),
        Conditional::Shatabdika => shatabdika_applies(facts.lagna),
        Conditional::ChaturaseetiSama => chaturaseeti_sama_applies(facts),
        Conditional::DwisaptatiSama => dwisaptati_sama_applies(facts),
        Conditional::Shashtihayani => shashtihayani_applies(facts.lagna, sun),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dasha::YEAR_365_25_NS;
    use crate::NAKSHATRA_SPAN;

    #[test]
    fn years_sum_to_the_named_cycles() {
        let want = [
            (Conditional::Dwadasottari, 112),
            (Conditional::Shodasottari, 116),
            (Conditional::Panchottari, 105),
            (Conditional::Shatabdika, 100),
            (Conditional::ChaturaseetiSama, 84),
            (Conditional::DwisaptatiSama, 72),
            (Conditional::Shashtihayani, 60),
        ];
        for (s, y) in want {
            assert_eq!(s.cycle_years(), y, "{}", s.name());
            assert_eq!(s.lords().len(), s.years().len(), "{}", s.name());
            // every lord occurs once
            for (i, l) in s.lords().iter().enumerate() {
                assert!(!s.lords()[i + 1..].contains(l), "{}", s.name());
            }
        }
        let mut seen = [0u8; 27];
        for (start, count) in SHASHTIHAYANI_GROUPS {
            for k in 0..count {
                seen[start + k] += 1;
            }
        }
        assert!(seen.iter().all(|c| *c == 1));
    }

    #[test]
    fn counting_origins_match_the_verses() {
        use DashaLord::*;
        let lord = |s: Conditional, n: usize| s.lords()[s.lord_index(n)];
        // Shodasottari: Pushya (count 1) → Sun; Ashlesha (2) → Mars; Vishakha (count 9,
        // remainder 1) → Sun; Punarvasu (count 27, remainder 3) → Jupiter.
        assert_eq!(lord(Conditional::Shodasottari, 7), Sun);
        assert_eq!(lord(Conditional::Shodasottari, 8), Mars);
        assert_eq!(lord(Conditional::Shodasottari, 15), Sun);
        assert_eq!(lord(Conditional::Shodasottari, 6), Jupiter);
        // Dwadasottari, janma → Revati: Revati (count 1) → Sun; U.Bhadrapada (2) →
        // Jupiter; Mula (9, remainder 1) → Sun; Ashwini (27, remainder 3) → Ketu.
        assert_eq!(lord(Conditional::Dwadasottari, 26), Sun);
        assert_eq!(lord(Conditional::Dwadasottari, 25), Jupiter);
        assert_eq!(lord(Conditional::Dwadasottari, 18), Sun);
        assert_eq!(lord(Conditional::Dwadasottari, 0), Ketu);
        // Panchottari: Anuradha → Sun, Jyeshtha → Mercury.
        assert_eq!(lord(Conditional::Panchottari, 16), Sun);
        assert_eq!(lord(Conditional::Panchottari, 17), Mercury);
        // Shatabdika: Revati → Sun, Ashwini → Moon.
        assert_eq!(lord(Conditional::Shatabdika, 26), Sun);
        assert_eq!(lord(Conditional::Shatabdika, 0), Moon);
        // Chaturaseeti-sama: Swati → Sun.
        assert_eq!(lord(Conditional::ChaturaseetiSama, 14), Sun);
        // Dwisaptati-sama: Mula → Sun, Revati (count 9) → Sun.
        assert_eq!(lord(Conditional::DwisaptatiSama, 18), Sun);
        assert_eq!(lord(Conditional::DwisaptatiSama, 26), Sun);
        // Shashtihayani: Ashwini → Jupiter, Rohini → Sun, Revati → Rahu.
        assert_eq!(lord(Conditional::Shashtihayani, 0), Jupiter);
        assert_eq!(lord(Conditional::Shashtihayani, 3), Sun);
        assert_eq!(lord(Conditional::Shashtihayani, 26), Rahu);
        assert_eq!(lord(Conditional::Shashtihayani, 18), Venus);
    }

    #[test]
    fn balance_full_at_nakshatra_start_and_cycle_is_contiguous() {
        let b = Instant::j2000();
        for s in Conditional::ALL {
            // Pushya starts at 7 × 13°20′: the lord of Pushya runs its full years.
            let v = conditional(s, 7.0 * NAKSHATRA_SPAN, b, YEAR_365_25_NS);
            let i = s.lord_index(7);
            assert_eq!(v[0].label, graha_name(s.lords()[i]), "{}", s.name());
            assert_eq!(v[0].start, b);
            assert_eq!(v[0].end.ns_since(b), s.years()[i] as i128 * YEAR_365_25_NS);
            for w in v.windows(2) {
                assert_eq!(w[0].end, w[1].start);
            }
            assert!(v.last().unwrap().end >= b.add_ns(120 * YEAR_365_25_NS));
            // one full cycle after the first period returns to the first lord
            let n = s.lords().len();
            assert_eq!(v[n].label, v[0].label);
            assert_eq!(
                v[n].start.ns_since(v[0].start),
                s.cycle_years() as i128 * YEAR_365_25_NS
            );
            // half-way through the nakshatra: half the years remain
            let v = conditional(s, 7.5 * NAKSHATRA_SPAN, b, YEAR_365_25_NS);
            let half = s.years()[i] as i128 * YEAR_365_25_NS / 2;
            assert!(v[0].end.ns_since(b).abs_diff(half) < 1_000, "{}", s.name());
        }
    }

    #[test]
    fn antardashas_partition_by_either_rule() {
        let b = Instant::j2000();
        let s = Conditional::Shatabdika;
        let v = conditional(s, 0.0, b, YEAR_365_25_NS); // Ashwini → Moon, 5 years
        assert_eq!(v[0].label, "Moon");
        let p = &v[1]; // Venus, 10 years
        assert_eq!(p.label, "Venus");
        let a = conditional_sub(s, p, AntarRule::Proportional);
        assert_eq!(a.len(), 7);
        assert_eq!(a[0].label, "Venus");
        assert_eq!(a[1].label, "Mercury");
        assert_eq!(a[6].label, "Moon");
        assert_eq!(a[0].start, p.start);
        assert_eq!(a[6].end, p.end);
        // Venus in Venus: 10 × 10 / 100 = 1 year
        assert_eq!(a[0].end.ns_since(a[0].start), YEAR_365_25_NS);
        let e = conditional_sub(s, p, AntarRule::Equal);
        assert_eq!(e.len(), 7);
        assert_eq!(e[0].end.ns_since(e[0].start), 10 * YEAR_365_25_NS / 7);
        for w in a.windows(2).chain(e.windows(2)) {
            assert_eq!(w[0].end, w[1].start);
        }
        // a sama dasha: both rules coincide
        let v = conditional(Conditional::DwisaptatiSama, 0.0, b, YEAR_365_25_NS);
        assert_eq!(
            conditional_sub(Conditional::DwisaptatiSama, &v[1], AntarRule::Proportional),
            conditional_sub(Conditional::DwisaptatiSama, &v[1], AntarRule::Equal)
        );
        // a label outside the system
        let mut q = v[1].clone();
        q.label = "Ketu".into();
        assert!(conditional_sub(Conditional::DwisaptatiSama, &q, AntarRule::Equal).is_empty());
    }

    #[test]
    fn applicability_conditions() {
        // Dwadasottari: 1° Taurus → navamsa Capricorn (no); 10° Aries → navamsa 4th from
        // Aries = Cancer (no); 5° Aries → navamsa Taurus (yes); 21° Aries → Libra (yes).
        assert!(!dwadasottari_applies(31.0));
        assert!(!dwadasottari_applies(10.0));
        assert!(dwadasottari_applies(5.0));
        assert!(dwadasottari_applies(21.0));
        // Shodasottari: 5° Aries is the Sun's hora; Moon 30° ahead of the Sun = bright.
        assert!(shodasottari_applies(5.0, 100.0, 130.0));
        assert!(!shodasottari_applies(5.0, 100.0, 300.0));
        // 20° Aries is the Moon's hora: applies in the dark fortnight.
        assert!(shodasottari_applies(20.0, 100.0, 300.0));
        assert!(!shodasottari_applies(20.0, 100.0, 130.0));
        // Panchottari: D12 Cancer. Aries 7.5°–10° → 4th dwadasamsa = Cancer; Cancer
        // 0°–2.5° → Cancer.
        assert!(panchottari_applies(8.0));
        assert!(panchottari_applies(91.0));
        assert!(!panchottari_applies(93.0));
        // Shatabdika: vargottama. 1° Aries (navamsa Aries) yes; 4° Aries no; Taurus
        // 13°20′–16°40′ (5th navamsa from Capricorn = Taurus) yes.
        assert!(shatabdika_applies(1.0));
        assert!(!shatabdika_applies(4.0));
        assert!(shatabdika_applies(45.0));
        // Chaturaseeti-sama: Aries lagna, Saturn (10th lord) in Capricorn.
        let mut f = ChartFacts {
            lagna: 5.0,
            grahas: [100.0, 130.0, 200.0, 110.0, 250.0, 70.0, 280.0, 20.0, 200.0],
        };
        assert!(chaturaseeti_sama_applies(&f));
        f.grahas[6] = 310.0;
        assert!(!chaturaseeti_sama_applies(&f));
        // Dwisaptati-sama: Aries lagna; Mars in Libra (lagna lord in 7th) → yes.
        assert!(dwisaptati_sama_applies(&f)); // Mars at 200° = Libra
        f.grahas[2] = 10.0;
        assert!(!dwisaptati_sama_applies(&f));
        // Venus (7th lord) in Aries → yes.
        f.grahas[5] = 15.0;
        assert!(dwisaptati_sama_applies(&f));
        // Shashtihayani: Sun in the lagna sign.
        assert!(shashtihayani_applies(5.0, 25.0));
        assert!(!shashtihayani_applies(5.0, 35.0));
        f.grahas[0] = 20.0;
        assert!(applies(Conditional::Shashtihayani, &f));
    }

    /// Kochi, 1990-02-20 05:04 UTC, 9.9312 N 76.2673 E, Lahiri. From the DE421 chart
    /// pinned in `kshana-chart` (`kochi_1990_02_20_birth_chart_and_dasha_balance`):
    /// Moon 244.505683° (4°30′20″ Sagittarius, Mula, index 18), lagna 36.958244° −
    /// ayanamsa 23.722934° = 13.235310° (13°14′ Aries), Sun 331.295533° − 23.722934° =
    /// 307.572599° (Aquarius).
    ///
    /// Hand calculation. Mula elapsed = 4.505683 / 13.333333 = 0.337926, so 0.662074 of
    /// the first lord's years remain:
    ///
    /// * Dwadasottari: Mula → Revati counts 9, 9 mod 8 = 1 → Sun 7 y; balance 4.6345 y.
    /// * Shodasottari: Pushya → Mula counts 12, 12 mod 8 = 4 → Saturn 14 y; 9.2690 y.
    /// * Panchottari: Anuradha → Mula counts 3 → Saturn 14 y; 9.2690 y.
    /// * Shatabdika: Revati → Mula counts 20, 20 mod 7 = 6 → Mars 20 y; 13.2415 y.
    /// * Chaturaseeti-sama: Swati → Mula counts 5 → Jupiter 12 y; 7.9449 y.
    /// * Dwisaptati-sama: Mula → Mula counts 1 → Sun 9 y; 5.9587 y.
    /// * Shashtihayani: Mula is in Venus's group (Jyeshtha–U.Ashadha) → Venus 6 y;
    ///   3.9724 y.
    ///
    /// Applicability: the lagna's navamsa is the 4th from Aries (10°–13°20′) = Cancer, so
    /// neither Dwadasottari (needs Taurus/Libra) nor Shatabdika (needs Aries); its hora is
    /// the Sun's (first half of an odd sign) and the Moon is 297° ahead of the Sun (dark
    /// fortnight, Krishna Dashami), so not Shodasottari; its dwadasamsa is the 6th from
    /// Aries (12°30′–15°) = Virgo, so not Panchottari; the Sun is in Aquarius, not the
    /// lagna sign, so not Shashtihayani.
    #[test]
    fn kochi_1990_first_periods_and_balances() {
        let moon = 244.505_683;
        let lagna = 13.235_310;
        let sun = 307.572_599;
        let b = Instant::j2000();
        let remain = 1.0 - (moon - 240.0) / NAKSHATRA_SPAN;
        assert!((remain - 0.662_074).abs() < 1e-6, "{remain}");
        let want = [
            (Conditional::Dwadasottari, "Sun", 7.0, "Jupiter", 4.6345),
            (Conditional::Shodasottari, "Saturn", 14.0, "Ketu", 9.2690),
            (Conditional::Panchottari, "Saturn", 14.0, "Mars", 9.2690),
            (Conditional::Shatabdika, "Mars", 20.0, "Saturn", 13.2415),
            (Conditional::ChaturaseetiSama, "Jupiter", 12.0, "Venus", 7.9449),
            (Conditional::DwisaptatiSama, "Sun", 9.0, "Moon", 5.9587),
            (Conditional::Shashtihayani, "Venus", 6.0, "Saturn", 3.9724),
        ];
        for (s, first, years, second, balance) in want {
            let v = conditional(s, moon, b, YEAR_365_25_NS);
            assert_eq!(v[0].label, first, "{}", s.name());
            assert_eq!(v[1].label, second, "{}", s.name());
            let got = v[0].end.ns_since(b) as f64 / YEAR_365_25_NS as f64;
            assert!((got - years * remain).abs() < 1e-6, "{} {got}", s.name());
            assert!((got - balance).abs() < 5e-5, "{} {got}", s.name());
        }
        assert!(!dwadasottari_applies(lagna));
        assert!(!shatabdika_applies(lagna));
        assert!(!is_shukla(sun, moon));
        assert!(!shodasottari_applies(lagna, sun, moon));
        assert!(!panchottari_applies(lagna));
        assert!(!shashtihayani_applies(lagna, sun));
    }
}
