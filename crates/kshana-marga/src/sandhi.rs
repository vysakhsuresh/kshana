//! Dasha sandhi and dasha chidra.
//!
//! # Sandhi
//!
//! A sandhi is the junction of two Vimshottari periods: the closing part of one and
//! the opening part of the next (BPHS, the dasha chapters, read the last antardasha of
//! a mahadasha as the turn towards the next; Wikipedia "Dasha (astrology)";
//! astrosight.ai "Dasha sandhi"). The texts give no fixed width, and the popular
//! figures (six months each side, a year, the two dashas' years in months) are
//! software habits. Two rules are offered and the one used is stated in every window:
//!
//! * [`SandhiRule::SubPeriod`] (the default): the last sub-period of the ending period
//!   and the first sub-period of the next, an exact and structural window (for a
//!   mahadasha boundary the two antardashas, for an antardasha boundary the two
//!   pratyantardashas).
//! * [`SandhiRule::Fraction`]: a stated fraction of each adjoining period, such as
//!   one-eighth, as some Kerala practitioners mark the sandhi; no text consulted fixes
//!   the fraction, so it is a parameter.
//!
//! # Chidra
//!
//! The chidra (vulnerability) lords read from a dasha lord (BPHS ch. 44, the marakas:
//! the lords of the 2nd and 7th, with the 12th and 8th as their companions; Prasna
//! Marga's chapter on longevity and the classical list of chhidra grahas that Jataka
//! Parijata, Phaladeepika and Prasna Marga share: the lord of the 22nd drekkana from
//! the lagna (Khara) and the lord of the 64th navamsa from the Moon; sitharsastrology
//! "The 64th navamsa and 22nd drekkana"; jyotish-blog.blogspot.com "64th Navamsa"):
//! the lords of the 2nd, 7th and 12th from the dasha lord's sign, the Khara drekkana
//! lord, the 64th navamsa lord, and every planet in the 8th from the dasha lord. The
//! 22nd drekkana is the drekkana of the 8th sign from the lagna with the lagna's own
//! ordinal, so its sign is the 8th from the lagna's drekkana sign; the 64th navamsa is
//! the Moon's navamsa ordinal carried seven signs on, so its sign is the 4th from the
//! Moon's navamsa sign. Convention: the 2nd, 7th and 12th are counted from the dasha
//! lord as the brief asks; the same lords counted from the lagna are the Parashari
//! marakas and are named in the reasons when they coincide.

use kshana_rules::Facts;
use kshana_time::Instant;
use kshana_vedic::dasha::YEAR_365_25_NS;
use kshana_vedic::{sign_lord, subdivide, varga, vimshottari, DashaLord, DashaPeriod};

use crate::sign_of;
use crate::sphuta::{gn, sn};
use kshana_rules::yogas::words;
use kshana_rules::{say3, Lang};

/// How wide a sandhi window is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SandhiRule {
    /// The last sub-period of the ending period and the first of the next.
    SubPeriod,
    /// This fraction of each adjoining period, e.g. `0.125` for an eighth.
    Fraction(f64),
}

impl SandhiRule {
    /// Name of the rule.
    pub fn name(self) -> &'static str {
        match self {
            SandhiRule::SubPeriod => "last and first sub-periods",
            SandhiRule::Fraction(_) => "fraction of each adjoining period",
        }
    }

    /// Name of the rule in the reader's language.
    pub fn name_in(self, lang: Lang) -> &'static str {
        match self {
            SandhiRule::SubPeriod => lang.pick([
                "last and first sub-periods",
                "അവസാനത്തെയും ആദ്യത്തെയും അപഹാരങ്ങൾ",
                "अंतिम और प्रथम अंतर्दशाएँ",
            ]),
            SandhiRule::Fraction(_) => lang.pick([
                "fraction of each adjoining period",
                "ചേർന്നുവരുന്ന ഓരോ ദശയുടെയും ഒരംശം",
                "दोनों सटी हुई दशाओं का एक अंश",
            ]),
        }
    }
}

/// One junction window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sandhi {
    /// 1 for a mahadasha boundary, 2 for an antardasha boundary inside a mahadasha.
    pub level: u8,
    /// Lord of the ending period.
    pub from_lord: DashaLord,
    /// Lord of the next period.
    pub to_lord: DashaLord,
    /// The exact boundary.
    pub boundary: Instant,
    /// Window start.
    pub start: Instant,
    /// Window end.
    pub end: Instant,
    /// The rule that set the width.
    pub rule: SandhiRule,
}

/// Mahadashas covering `[from, to)`, cycles repeated as needed.
fn mahadashas_covering(
    moon_sidereal: f64,
    birth: Instant,
    year_ns: i128,
    to: Instant,
) -> Vec<DashaPeriod> {
    let cycle = vimshottari(moon_sidereal, birth, year_ns);
    let len = 120 * year_ns;
    let mut out = Vec::new();
    let mut shift: i128 = 0;
    loop {
        for p in &cycle {
            out.push(DashaPeriod {
                lord: p.lord,
                start: p.start.add_ns(shift),
                end: p.end.add_ns(shift),
            });
        }
        if out.last().map(|p| p.end >= to).unwrap_or(true) || shift > 20 * len {
            break;
        }
        shift += len;
    }
    out
}

fn window(rule: SandhiRule, ending: DashaPeriod, next: DashaPeriod) -> (Instant, Instant) {
    match rule {
        SandhiRule::SubPeriod => {
            let last = subdivide(ending)[8];
            let first = subdivide(next)[0];
            (last.start, first.end)
        }
        SandhiRule::Fraction(f) => {
            let f = f.clamp(0.0, 1.0);
            let a = (ending.end.ns_since(ending.start) as f64 * f) as i128;
            let b = (next.end.ns_since(next.start) as f64 * f) as i128;
            (ending.end.add_ns(-a), next.start.add_ns(b))
        }
    }
}

/// Every level-1 and level-2 sandhi whose boundary lies in `[from, to]`, in time
/// order, under the default rule ([`SandhiRule::SubPeriod`]) and the 365.25-day year.
pub fn sandhi_windows(
    moon_sidereal: f64,
    birth: Instant,
    from: Instant,
    to: Instant,
) -> Vec<Sandhi> {
    sandhi_windows_with(
        moon_sidereal,
        birth,
        from,
        to,
        SandhiRule::SubPeriod,
        YEAR_365_25_NS,
    )
}

/// Every level-1 and level-2 sandhi whose boundary lies in `[from, to]` under a stated
/// rule and year length.
pub fn sandhi_windows_with(
    moon_sidereal: f64,
    birth: Instant,
    from: Instant,
    to: Instant,
    rule: SandhiRule,
    year_ns: i128,
) -> Vec<Sandhi> {
    let mahas = mahadashas_covering(moon_sidereal, birth, year_ns, to);
    let mut out = Vec::new();
    for w in mahas.windows(2) {
        let b = w[0].end;
        if b >= from && b <= to {
            let (start, end) = window(rule, w[0], w[1]);
            out.push(Sandhi {
                level: 1,
                from_lord: w[0].lord,
                to_lord: w[1].lord,
                boundary: b,
                start,
                end,
                rule,
            });
        }
    }
    for m in &mahas {
        if m.end < from || m.start > to {
            continue;
        }
        let antars = subdivide(*m);
        for w in antars.windows(2) {
            let b = w[0].end;
            if b >= from && b <= to {
                let (start, end) = window(rule, w[0], w[1]);
                out.push(Sandhi {
                    level: 2,
                    from_lord: w[0].lord,
                    to_lord: w[1].lord,
                    boundary: b,
                    start,
                    end,
                    rule,
                });
            }
        }
    }
    out.sort_by_key(|s| (s.boundary, s.level));
    out
}

/// Lord of the 22nd (Khara) drekkana from the lagna: the drekkana of the point seven
/// signs on from the lagna, which is the 8th sign from the lagna's drekkana sign.
pub fn khara_drekkana_lord(lagna_sidereal: f64) -> DashaLord {
    sign_lord(sign_of(varga(lagna_sidereal + 210.0, 3)) as usize)
}

/// Lord of the 64th navamsa from the Moon: the navamsa of the point seven signs on
/// from the Moon, which is the 4th sign from the Moon's navamsa sign.
pub fn navamsa_64_lord(moon_sidereal: f64) -> DashaLord {
    sign_lord(sign_of(varga(moon_sidereal + 210.0, 9)) as usize)
}

/// The chidra lords of a dasha lord, each with its reason.
pub fn chidra_lords(facts: &Facts, dasha_lord: DashaLord) -> Vec<(DashaLord, String)> {
    chidra_lords_in(facts, dasha_lord, Lang::En)
}

/// [`chidra_lords`] with the reasons in the reader's language.
pub fn chidra_lords_in(facts: &Facts, dasha_lord: DashaLord, lang: Lang) -> Vec<(DashaLord, String)> {
    let dl = facts.planet(dasha_lord);
    let from = dl.sign;
    let lagna = facts.lagna_sign;
    let dln = gn(lang, dasha_lord);
    let mut out = Vec::new();
    for (h, what) in [(2u8, "2nd"), (7, "7th"), (12, "12th")] {
        let sign = (from + h - 1) % 12;
        let lord = sign_lord(sign as usize);
        let from_lagna = ((sign as i32 - lagna as i32).rem_euclid(12) + 1) as u8;
        let maraka = if matches!(from_lagna, 2 | 7) {
            say3!(
                lang,
                "; also the lord of the {from_lagna}th from the lagna, a Parashari maraka",
                "; ലഗ്നത്തിൽ നിന്ന് {from_lagna}-ാം ഭാവത്തിന്റെ അധിപനും, പാരാശരി രീതിയിൽ ക്ലേശകാരകൻ",
                "; लग्न से {from_lagna}वें भाव का स्वामी भी, पाराशरी पद्धति में कष्टकारक"
            )
        } else {
            String::new()
        };
        let (s, f) = (sn(lang, sign), sn(lang, from));
        let ord = match lang {
            Lang::En => what,
            _ => words::bhava(lang, h),
        };
        out.push((
            lord,
            say3!(
                lang,
                "lord of the {ord} from {dln} ({s} from {f}){maraka}",
                "ദശാനാഥനിൽ ({dln}) നിന്ന് {ord} ഭാവത്തിന്റെ അധിപൻ ({f} രാശിയിൽ നിന്ന് {s}){maraka}",
                "{dln} से {ord} भाव का स्वामी ({f} से {s}){maraka}"
            ),
        ));
    }
    let khara = khara_drekkana_lord(facts.cusps[0]);
    let ks = sn(lang, sign_of(varga(facts.cusps[0] + 210.0, 3)));
    out.push((
        khara,
        say3!(
            lang,
            "lord of the 22nd (Khara) drekkana from the lagna, {ks}",
            "ലഗ്നത്തിൽ നിന്ന് 22-ാം (ഖര) ദ്രേക്കാണത്തിന്റെ അധിപൻ, {ks}",
            "लग्न से 22वें (खर) द्रेष्काण का स्वामी, {ks}"
        ),
    ));
    let moon = facts.planet(DashaLord::Moon).sidereal;
    let n64 = navamsa_64_lord(moon);
    let ns = sn(lang, sign_of(varga(moon + 210.0, 9)));
    out.push((
        n64,
        say3!(
            lang,
            "lord of the 64th navamsa from the Moon, {ns}",
            "ചന്ദ്രനിൽ നിന്ന് 64-ാം നവാംശത്തിന്റെ അധിപൻ, {ns}",
            "चंद्र से 64वें नवांश का स्वामी, {ns}"
        ),
    ));
    let eighth = (from + 7) % 12;
    let es = sn(lang, eighth);
    for p in &facts.planets {
        if p.sign == eighth {
            out.push((
                p.graha,
                say3!(
                    lang,
                    "in the 8th from {dln} ({es})",
                    "ദശാനാഥനിൽ ({dln}) നിന്ന് എട്ടാമത് ({es})",
                    "{dln} से आठवें में ({es})"
                ),
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::{kernel, kochi_birth, kochi_chart};
    use kshana_time::NS_PER_DAY;

    #[test]
    fn windows_bracket_each_boundary() {
        let birth = Instant::j2000();
        let moon = 243.2; // Mula: Ketu first
        let from = birth;
        let to = birth.add_ns(45 * YEAR_365_25_NS);
        let v = vimshottari(moon, birth, YEAR_365_25_NS);
        let s = sandhi_windows(moon, birth, from, to);
        // level 1: the Ketu→Venus, Venus→Sun, Sun→Moon and Moon→Mars boundaries
        let l1: Vec<&Sandhi> = s.iter().filter(|x| x.level == 1).collect();
        assert_eq!(l1.len(), 4, "{l1:?}");
        assert_eq!(
            (l1[0].from_lord, l1[0].to_lord),
            (DashaLord::Ketu, DashaLord::Venus)
        );
        assert_eq!(l1[0].boundary, v[0].end);
        // the window is the last antardasha of Ketu and the first of Venus
        let ketu_antars = subdivide(v[0]);
        let venus_antars = subdivide(v[1]);
        assert_eq!(l1[0].start, ketu_antars[8].start);
        assert_eq!(l1[0].end, venus_antars[0].end);
        for x in &s {
            assert!(x.start < x.boundary && x.boundary < x.end, "{x:?}");
            assert!(x.boundary >= from && x.boundary <= to);
            assert!(x.level == 1 || x.level == 2);
        }
        // level 2: eight inner boundaries per mahadasha
        let l2 = s.iter().filter(|x| x.level == 2).count();
        assert!(l2 >= 4 * 8, "{l2}");
        // level-2 windows are pratyantar-sized, shorter than level-1 ones
        let l2_first = s.iter().find(|x| x.level == 2).unwrap();
        assert!(l2_first.end.ns_since(l2_first.start) < l1[0].end.ns_since(l1[0].start));
        // sorted
        assert!(s.windows(2).all(|w| w[0].boundary <= w[1].boundary));
        // the fraction rule: an eighth of each side
        let f = sandhi_windows_with(
            moon,
            birth,
            from,
            to,
            SandhiRule::Fraction(0.125),
            YEAR_365_25_NS,
        );
        let x = f.iter().find(|x| x.level == 1).unwrap();
        assert_eq!(
            x.boundary.ns_since(x.start),
            (v[0].end.ns_since(v[0].start) as f64 * 0.125) as i128
        );
        assert_eq!(
            x.end.ns_since(x.boundary),
            (20 * YEAR_365_25_NS) as f64 as i128 / 8
        );
        assert_eq!(x.rule.name(), "fraction of each adjoining period");
        // a range beyond the first cycle still finds boundaries
        let far = sandhi_windows(
            moon,
            birth,
            birth.add_ns(125 * YEAR_365_25_NS),
            birth.add_ns(135 * YEAR_365_25_NS),
        );
        assert!(far.iter().any(|x| x.level == 1));
        assert!(far
            .iter()
            .all(|x| x.boundary > birth.add_ns(125 * YEAR_365_25_NS - NS_PER_DAY)));
    }

    #[test]
    fn khara_and_sixty_fourth() {
        // lagna 5° Aries: first drekkana Aries; the 22nd is the first drekkana of Scorpio → Scorpio, Mars
        assert_eq!(khara_drekkana_lord(5.0), DashaLord::Mars);
        // lagna 15° Aries: second drekkana Leo; 22nd: second drekkana of Scorpio → Pisces, Jupiter
        assert_eq!(khara_drekkana_lord(15.0), DashaLord::Jupiter);
        // the 22nd drekkana sign is the 8th from the lagna's drekkana sign
        for i in 0..36 {
            let l = i as f64 * 10.0 + 3.0;
            let d = sign_of(varga(l, 3));
            assert_eq!(sign_of(varga(l + 210.0, 3)), (d + 7) % 12, "{l}");
        }
        // the 64th navamsa sign is the 4th from the Moon's navamsa sign
        for i in 0..108 {
            let m = i as f64 * 360.0 / 108.0 + 1.0;
            let n = sign_of(varga(m, 9));
            assert_eq!(sign_of(varga(m + 210.0, 9)), (n + 3) % 12, "{m}");
        }
        // Moon 1° Aries: navamsa Aries; 64th: Cancer → Moon
        assert_eq!(navamsa_64_lord(1.0), DashaLord::Moon);
    }

    #[test]
    fn chidra_lords_count_and_reasons() {
        let Some(spk) = kernel() else { return };
        let ch = kochi_chart(&spk);
        let f = Facts::from_chart(&ch);
        for lord in DashaLord::ORDER {
            let c = chidra_lords(&f, lord);
            let eighth = (f.planet(lord).sign + 7) % 12;
            let in_eighth = f.planets.iter().filter(|p| p.sign == eighth).count();
            assert_eq!(c.len(), 5 + in_eighth, "{lord:?}: {c:?}");
            assert!(c.iter().all(|(_, r)| !r.is_empty()));
            assert_eq!(c[3].0, khara_drekkana_lord(ch.cusps_sidereal[0]));
            assert_eq!(c[4].0, navamsa_64_lord(ch.bodies[1].sidereal));
            assert!(c[0].1.starts_with("lord of the 2nd"));
        }
        // and the sandhi of the Kochi birth over its first forty years
        let moon = ch.bodies[1].sidereal;
        let s = sandhi_windows(
            moon,
            kochi_birth(),
            kochi_birth(),
            kochi_birth().add_ns(40 * YEAR_365_25_NS),
        );
        assert!(s.iter().filter(|x| x.level == 1).count() >= 2);
        assert!(s.iter().all(|x| x.start < x.boundary && x.boundary < x.end));
    }
}
