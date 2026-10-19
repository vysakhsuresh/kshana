//! Ayurdaya: the three classical longevity arithmetics of Brihat Parashara Hora
//! Shastra ch. 43 (R. Santhanam's translation, Ranjan, 1984; the chapter numbering of
//! the bphs.blogspot.com transcription), Pindayu, Nisargayu and Amsayu, with the four
//! haranas, the bharanas, the lagna's share and the rule that picks the method.
//!
//! **This is a classical exercise.** Every result carries [`LONGEVITY_CAVEAT`] in its
//! words: the app presents the arithmetic as the text's method, never as a prediction
//! of death.
//!
//! # Sources and conventions
//!
//! * Pindayu years (BPHS 43.2–3; Jataka Parijata V): Sun 19, Moon 25, Mars 15, Mercury
//!   12, Jupiter 15, Venus 21, Saturn 20, the full term at deep exaltation, half at deep
//!   debilitation, proportionate between (BPHS 43.4–5, "by the rule of three";
//!   webastrologers.com "Age increasing by a planet as per Rishi Parashar"):
//!   `years = full × (180° + arc) / 360°`, `arc` being the planet's distance from its
//!   deep debilitation folded to at most 180° ([`proportioned_years`]).
//! * Nisargayu years (BPHS 43.?, the natural years; medium.com/thoughts-on-jyotish
//!   "Pindayu, Nisargayu and Amsayu"): Sun 20, Moon 1, Mars 2, Mercury 9, Jupiter 18,
//!   Venus 20, Saturn 50 (120 in all). Convention: the same exaltation proportion and
//!   the same haranas as Pindayu are applied, as the commentators do; some apply the
//!   natural years whole.
//! * Amsayu (BPHS 43.15–22; webastrologers.com "What is Ansayu"): each planet's and the
//!   lagna's longitude in minutes divided by 200 (one navamsa), the remainder on
//!   division by twelve being the years, so a navamsa cycle is twelve years. Bharanas:
//!   trebled in exaltation or retrogression, doubled in own navamsa, own drekkana or
//!   vargottama, only the trebling when both apply (BPHS 43.22). The haranas apply
//!   except Krurodaya (the lagna is not a planet).
//! * The four haranas (BPHS 43.8–14; Jataka Parijata V; medium.com "Pindayu, Nisargayu
//!   and Amsayu"): Chakrapata (Vyayadi), a planet in the 12th loses all, 11th half,
//!   10th a third, 9th a fourth, 8th a fifth, 7th a sixth, benefics half of those, and
//!   when several planets share a house only the strongest loses; Satrukshetra, a
//!   planet in an enemy's sign loses a third, retrograde planets and Mars excepted;
//!   Astangata, a combust planet loses half, Venus and Saturn excepted; Krurodaya, with
//!   a malefic (Sun, Mars or Saturn) in the lagna the total is reduced by the lagna's
//!   degrees over 108, halved when a benefic aspects or occupies the lagna, applied
//!   after the other three on the sum (Jataka Parijata 5.13). The haranas multiply the
//!   remainder in turn, so their order does not matter. Conventions: the houses of the
//!   Chakrapata harana are 30° arcs from the lagna's degree (Santhanam's "by degree
//!   positions, not bhava placement"); "strongest" among planets sharing a house is the
//!   one nearest its exaltation, since a Shadbala is not needed elsewhere here.
//! * The lagna's share (BPHS 43.15; Jataka Parijata): for Pindayu and Nisargayu the
//!   signs elapsed from Aries with the fraction of the rising sign, for Amsayu its
//!   navamsa count like a planet's. Convention: some authors use the navamsa count in
//!   Pindayu too when the navamsa lord is the stronger; the rasi count is used here.
//! * Selection (BPHS 43.1, 43.24–25): the strongest of the lagna, the Sun and the Moon
//!   chooses Amsayu, Pindayu or Nisargayu respectively; two equally strong, the average
//!   of their two methods; all three, the average of all.
//! * Paramayu: the text's limit of 120 years for a human life (BPHS 43, the seven
//!   grades of longevity); totals above it are clamped and the clamp is stated.

use kshana_bala::tables::{combustion_orb, deep_debilitation, SEVEN};
use kshana_chart::Chart;
use kshana_ephem::Spk;
use kshana_rules::parashari::{aspected_houses, dignity, is_benefic, Dignity};
use kshana_rules::Facts;
use kshana_sky::Observer;
use kshana_time::EopTable;
use kshana_vedic::{sign_lord, varga, Ayanamsa, DashaLord};

use crate::{body_sidereal, graha_name, norm360, sign_of, MargaError, SIGN_NAMES};

/// The sentence every ayurdaya result carries.
pub const LONGEVITY_CAVEAT: &str = "Longevity arithmetic is a classical exercise of the text; the app presents it as such, as an illustration of the method, and never as a prediction of death.";

/// The text's limit of a human life, years.
pub const PARAMAYU: f64 = 120.0;

/// Which arithmetic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AyurMethod {
    /// The Sun's method: planetary years by exaltation.
    Pindayu,
    /// The Moon's method: the natural years.
    Nisargayu,
    /// The lagna's method: navamsa counts.
    Amsayu,
    /// The mean of two or three methods that tied.
    Average,
}

impl AyurMethod {
    /// Name.
    pub fn name(self) -> &'static str {
        match self {
            AyurMethod::Pindayu => "Pindayu",
            AyurMethod::Nisargayu => "Nisargayu",
            AyurMethod::Amsayu => "Amsayu",
            AyurMethod::Average => "Average",
        }
    }
}

/// Pindayu full years (BPHS 43.2–3). Nodes give 0.
pub fn pindayu_years(g: DashaLord) -> f64 {
    match g {
        DashaLord::Sun => 19.0,
        DashaLord::Moon => 25.0,
        DashaLord::Mars => 15.0,
        DashaLord::Mercury => 12.0,
        DashaLord::Jupiter => 15.0,
        DashaLord::Venus => 21.0,
        DashaLord::Saturn => 20.0,
        _ => 0.0,
    }
}

/// Nisargayu natural years (BPHS 43). Nodes give 0.
pub fn nisargayu_years(g: DashaLord) -> f64 {
    match g {
        DashaLord::Sun => 20.0,
        DashaLord::Moon => 1.0,
        DashaLord::Mars => 2.0,
        DashaLord::Mercury => 9.0,
        DashaLord::Jupiter => 18.0,
        DashaLord::Venus => 20.0,
        DashaLord::Saturn => 50.0,
        _ => 0.0,
    }
}

/// Distance of a graha from its deep debilitation, folded to `[0, 180]`: 180 at deep
/// exaltation. Nodes give 0.
pub fn exaltation_arc(g: DashaLord, sidereal: f64) -> f64 {
    let Some(d) = deep_debilitation(g) else {
        return 0.0;
    };
    let arc = (sidereal - d).rem_euclid(360.0);
    if arc > 180.0 {
        360.0 - arc
    } else {
        arc
    }
}

/// Full years proportioned by the exaltation arc: the full term at 180°, half at 0°.
pub fn proportioned_years(full: f64, arc: f64) -> f64 {
    full * (180.0 + arc.clamp(0.0, 180.0)) / 360.0
}

/// One harana or bharana applied to a contribution.
#[derive(Clone, Debug, PartialEq)]
pub struct Reduction {
    /// Which rule.
    pub name: &'static str,
    /// The factor the remainder was multiplied by (for a bharana, 2 or 3).
    pub factor: f64,
    /// Years taken away (negative when added).
    pub years: f64,
    /// Why, in words.
    pub reason: String,
}

/// One graha's (or the lagna's) share.
#[derive(Clone, Debug, PartialEq)]
pub struct Contribution {
    /// "Sun" … "Saturn", or "Lagna".
    pub graha: &'static str,
    /// Years before any harana or bharana.
    pub base_years: f64,
    /// Years after them.
    pub years: f64,
    /// The haranas and bharanas applied, in order.
    pub reductions: Vec<Reduction>,
    /// The arithmetic, in words.
    pub words: Vec<String>,
}

/// One longevity reckoning.
#[derive(Clone, Debug, PartialEq)]
pub struct Ayurdaya {
    /// The method.
    pub method: AyurMethod,
    /// Years, clamped to `[0, PARAMAYU]`.
    pub years: f64,
    /// Years before the clamp.
    pub raw_years: f64,
    /// The seven grahas and the lagna.
    pub contributions: Vec<Contribution>,
    /// The Krurodaya harana on the total, when it applied.
    pub krurodaya: Option<Reduction>,
    /// Why this method was chosen.
    pub selection: String,
    /// The caveat, the selection and the arithmetic, in words.
    pub words: Vec<String>,
    /// The methods averaged, when `method` is [`AyurMethod::Average`].
    pub components: Vec<Ayurdaya>,
}

/// One planet as the arithmetic sees it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AyurPlanet {
    /// Which.
    pub graha: DashaLord,
    /// Sidereal longitude.
    pub sidereal: f64,
    /// Retrograde.
    pub retrograde: bool,
    /// Natural benefic (Jupiter, Venus, unafflicted Mercury, the waxing Moon).
    pub benefic: bool,
    /// Within its combustion orb of the Sun.
    pub combust: bool,
}

/// What the arithmetic needs.
#[derive(Clone, Debug, PartialEq)]
pub struct AyurInput {
    /// Sidereal lagna.
    pub lagna: f64,
    /// The seven grahas, Sun to Saturn.
    pub planets: Vec<AyurPlanet>,
    /// A benefic occupies or aspects the lagna (halves the Krurodaya harana).
    pub benefic_on_lagna: bool,
}

impl AyurInput {
    /// Read the input from a chart.
    pub fn from_chart(chart: &Chart) -> Result<Self, MargaError> {
        let facts = Facts::from_chart(chart);
        let sun = body_sidereal(chart, "Sun")?;
        let mut planets = Vec::with_capacity(7);
        for g in SEVEN {
            let name = graha_name(g);
            let b = chart
                .bodies
                .iter()
                .find(|b| b.name == name)
                .ok_or(MargaError::MissingBody(name))?;
            let retrograde = b.speed < 0.0;
            let sep = {
                let d = (b.sidereal - sun).rem_euclid(360.0);
                if d > 180.0 {
                    360.0 - d
                } else {
                    d
                }
            };
            let combust =
                g != DashaLord::Sun && combustion_orb(g, retrograde).is_some_and(|orb| sep < orb);
            planets.push(AyurPlanet {
                graha: g,
                sidereal: b.sidereal,
                retrograde,
                benefic: is_benefic(&facts, g),
                combust,
            });
        }
        let benefic_on_lagna = facts.planets.iter().any(|p| {
            is_benefic(&facts, p.graha)
                && (p.rasi_house == 1 || aspected_houses(p.graha, p.rasi_house).contains(&1))
        });
        Ok(Self {
            lagna: chart.cusps_sidereal[0],
            planets,
            benefic_on_lagna,
        })
    }
}

/// House 1–12 by 30° arcs from the lagna's degree.
fn chakrapata_house(lagna: f64, lon: f64) -> u8 {
    ((lon - lagna).rem_euclid(360.0) / 30.0).floor() as u8 % 12 + 1
}

/// Fraction lost in the Chakrapata harana for a malefic in `house`.
fn chakrapata_fraction(house: u8) -> f64 {
    match house {
        12 => 1.0,
        11 => 1.0 / 2.0,
        10 => 1.0 / 3.0,
        9 => 1.0 / 4.0,
        8 => 1.0 / 5.0,
        7 => 1.0 / 6.0,
        _ => 0.0,
    }
}

/// The haranas common to the three methods, applied in turn to `years`.
fn haranas(
    input: &AyurInput,
    p: &AyurPlanet,
    strongest_in_house: bool,
    mut years: f64,
    out: &mut Vec<Reduction>,
) -> f64 {
    let house = chakrapata_house(input.lagna, p.sidereal);
    let mut f = chakrapata_fraction(house);
    if p.benefic {
        f /= 2.0;
    }
    if f > 0.0 {
        if strongest_in_house {
            let lost = years * f;
            out.push(Reduction {
                name: "Chakrapata harana",
                factor: 1.0 - f,
                years: lost,
                reason: format!(
                    "in the {house}th from the lagna's degree ({}), loses {:.3} of its years",
                    if p.benefic {
                        "benefic, half rate"
                    } else {
                        "malefic"
                    },
                    f
                ),
            });
            years -= lost;
        } else {
            out.push(Reduction {
                name: "Chakrapata harana",
                factor: 1.0,
                years: 0.0,
                reason: format!(
                    "in the {house}th from the lagna's degree but not the strongest planet there; only the strongest loses"
                ),
            });
        }
    }
    if dignity(p.graha, p.sidereal) == Dignity::Enemy {
        if p.retrograde || p.graha == DashaLord::Mars {
            out.push(Reduction {
                name: "Satrukshetra harana",
                factor: 1.0,
                years: 0.0,
                reason: format!(
                    "in an enemy's sign but {}: excepted",
                    if p.retrograde { "retrograde" } else { "Mars" }
                ),
            });
        } else {
            let lost = years / 3.0;
            out.push(Reduction {
                name: "Satrukshetra harana",
                factor: 2.0 / 3.0,
                years: lost,
                reason: "in an enemy's sign: loses a third".into(),
            });
            years -= lost;
        }
    }
    if p.combust {
        if matches!(p.graha, DashaLord::Venus | DashaLord::Saturn) {
            out.push(Reduction {
                name: "Astangata harana",
                factor: 1.0,
                years: 0.0,
                reason: "combust, but Venus and Saturn are excepted".into(),
            });
        } else {
            let lost = years / 2.0;
            out.push(Reduction {
                name: "Astangata harana",
                factor: 0.5,
                years: lost,
                reason: "combust: loses half".into(),
            });
            years -= lost;
        }
    }
    years
}

/// Is `p` the strongest (nearest its exaltation) of the planets sharing its
/// Chakrapata house?
fn strongest_in_house(input: &AyurInput, p: &AyurPlanet) -> bool {
    let h = chakrapata_house(input.lagna, p.sidereal);
    let mine = exaltation_arc(p.graha, p.sidereal);
    input
        .planets
        .iter()
        .filter(|q| chakrapata_house(input.lagna, q.sidereal) == h && q.graha != p.graha)
        .all(|q| exaltation_arc(q.graha, q.sidereal) <= mine)
}

fn navamsa_years(lon: f64) -> f64 {
    (norm360(lon) * 60.0 / 200.0).rem_euclid(12.0)
}

fn one_method(input: &AyurInput, method: AyurMethod) -> Ayurdaya {
    let mut contributions = Vec::with_capacity(8);
    for p in &input.planets {
        let mut reductions = Vec::new();
        let mut words = Vec::new();
        let (base, base_words) = match method {
            AyurMethod::Amsayu => {
                let y = navamsa_years(p.sidereal);
                (
                    y,
                    format!(
                        "{} at {:.4}°: {:.2} navamsas, remainder by twelve {:.4} years",
                        graha_name(p.graha),
                        p.sidereal,
                        norm360(p.sidereal) * 60.0 / 200.0,
                        y
                    ),
                )
            }
            _ => {
                let full = if method == AyurMethod::Pindayu {
                    pindayu_years(p.graha)
                } else {
                    nisargayu_years(p.graha)
                };
                let arc = exaltation_arc(p.graha, p.sidereal);
                let y = proportioned_years(full, arc);
                (
                    y,
                    format!(
                        "{} at {:.4}°: {full} full years, {arc:.2}° from deep debilitation, gives {y:.4} years",
                        graha_name(p.graha),
                        p.sidereal
                    ),
                )
            }
        };
        words.push(base_words);
        let mut years = base;
        if method == AyurMethod::Amsayu {
            let exalted = dignity(p.graha, p.sidereal) == Dignity::Exalted;
            let nav = sign_of(varga(p.sidereal, 9));
            let drek = sign_of(varga(p.sidereal, 3));
            let own_nav = sign_lord(nav as usize) == p.graha;
            let own_drek = sign_lord(drek as usize) == p.graha;
            let vargottama = nav == sign_of(p.sidereal);
            if exalted || p.retrograde {
                reductions.push(Reduction {
                    name: "Bharana",
                    factor: 3.0,
                    years: -2.0 * years,
                    reason: format!(
                        "{}: trebled{}",
                        if exalted { "exalted" } else { "retrograde" },
                        if own_nav || own_drek || vargottama {
                            " (the doubling would also apply; only the trebling is taken)"
                        } else {
                            ""
                        }
                    ),
                });
                years *= 3.0;
            } else if own_nav || own_drek || vargottama {
                reductions.push(Reduction {
                    name: "Bharana",
                    factor: 2.0,
                    years: -years,
                    reason: format!(
                        "{}: doubled",
                        if vargottama {
                            "vargottama"
                        } else if own_nav {
                            "in its own navamsa"
                        } else {
                            "in its own drekkana"
                        }
                    ),
                });
                years *= 2.0;
            }
        }
        years = haranas(
            input,
            p,
            strongest_in_house(input, p),
            years,
            &mut reductions,
        );
        for r in &reductions {
            words.push(format!("{}: {} ({:+.4} years)", r.name, r.reason, -r.years));
        }
        words.push(format!("{}: {years:.4} years", graha_name(p.graha)));
        contributions.push(Contribution {
            graha: graha_name(p.graha),
            base_years: base,
            years,
            reductions,
            words,
        });
    }
    let (lagna_years, lagna_words) = match method {
        AyurMethod::Amsayu => {
            let y = navamsa_years(input.lagna);
            (
                y,
                format!(
                    "Lagna at {:.4}°: {:.2} navamsas, remainder by twelve {y:.4} years",
                    input.lagna,
                    norm360(input.lagna) * 60.0 / 200.0
                ),
            )
        }
        _ => {
            let y = norm360(input.lagna) / 30.0;
            (
                y,
                format!(
                    "Lagna at {:.4}° ({}): {y:.4} signs elapsed from Aries, {y:.4} years",
                    input.lagna,
                    SIGN_NAMES[sign_of(input.lagna) as usize]
                ),
            )
        }
    };
    contributions.push(Contribution {
        graha: "Lagna",
        base_years: lagna_years,
        years: lagna_years,
        reductions: Vec::new(),
        words: vec![lagna_words],
    });
    let sum: f64 = contributions.iter().map(|c| c.years).sum();
    let mut raw = sum;
    let mut krurodaya = None;
    if method != AyurMethod::Amsayu {
        let lagna_sign = sign_of(input.lagna);
        let malefics: Vec<&AyurPlanet> = input
            .planets
            .iter()
            .filter(|p| {
                matches!(
                    p.graha,
                    DashaLord::Sun | DashaLord::Mars | DashaLord::Saturn
                ) && sign_of(p.sidereal) == lagna_sign
            })
            .collect();
        if !malefics.is_empty() {
            let deg = norm360(input.lagna).rem_euclid(30.0);
            let mut f = deg / 108.0;
            if input.benefic_on_lagna {
                f /= 2.0;
            }
            let lost = sum * f;
            krurodaya = Some(Reduction {
                name: "Krurodaya harana",
                factor: 1.0 - f,
                years: lost,
                reason: format!(
                    "{} in the lagna sign: the total is reduced by the lagna's {deg:.2}° over 108{}",
                    malefics
                        .iter()
                        .map(|p| graha_name(p.graha))
                        .collect::<Vec<_>>()
                        .join(", "),
                    if input.benefic_on_lagna {
                        ", halved because a benefic occupies or aspects the lagna"
                    } else {
                        ""
                    }
                ),
            });
            raw -= lost;
        }
    }
    let years = raw.clamp(0.0, PARAMAYU);
    let mut words = vec![
        LONGEVITY_CAVEAT.to_string(),
        format!(
            "{}: the seven grahas and the lagna give {sum:.4} years before the Krurodaya harana",
            method.name()
        ),
    ];
    if let Some(k) = &krurodaya {
        words.push(format!("{}: {} (−{:.4} years)", k.name, k.reason, k.years));
    }
    if raw > PARAMAYU {
        words.push(format!(
            "{raw:.4} years exceeds the text's Paramayu of {PARAMAYU} years and is clamped to it"
        ));
    } else if raw < 0.0 {
        words.push(format!(
            "{raw:.4} years is below zero and is clamped to zero"
        ));
    }
    words.push(format!("{}: {years:.4} years", method.name()));
    Ayurdaya {
        method,
        years,
        raw_years: raw,
        contributions,
        krurodaya,
        selection: String::new(),
        words,
        components: Vec::new(),
    }
}

/// One method's reckoning. [`AyurMethod::Average`] averages all three.
pub fn ayurdaya_by(input: &AyurInput, method: AyurMethod) -> Ayurdaya {
    match method {
        AyurMethod::Average => average(
            ayurdaya_all(input).to_vec(),
            "all three methods averaged".into(),
        ),
        m => one_method(input, m),
    }
}

/// Pindayu, Nisargayu and Amsayu, in that order.
pub fn ayurdaya_all(input: &AyurInput) -> [Ayurdaya; 3] {
    [
        one_method(input, AyurMethod::Pindayu),
        one_method(input, AyurMethod::Nisargayu),
        one_method(input, AyurMethod::Amsayu),
    ]
}

fn average(components: Vec<Ayurdaya>, selection: String) -> Ayurdaya {
    let n = components.len().max(1) as f64;
    let raw = components.iter().map(|c| c.years).sum::<f64>() / n;
    let years = raw.clamp(0.0, PARAMAYU);
    let mut words = vec![LONGEVITY_CAVEAT.to_string(), selection.clone()];
    for c in &components {
        words.push(format!("{}: {:.4} years", c.method.name(), c.years));
    }
    words.push(format!("Average: {years:.4} years"));
    Ayurdaya {
        method: AyurMethod::Average,
        years,
        raw_years: raw,
        contributions: Vec::new(),
        krurodaya: None,
        selection,
        words,
        components,
    }
}

/// Strengths of the three selectors on one scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Strengths {
    /// The Sun's strength.
    pub sun: f64,
    /// The Moon's strength.
    pub moon: f64,
    /// The lagna's strength.
    pub lagna: f64,
}

/// The methods the strengths select: the strongest selector's method, or every method
/// whose selector ties for the strongest (BPHS 43.24–25).
pub fn select_methods(s: &Strengths) -> Vec<AyurMethod> {
    let top = s.sun.max(s.moon).max(s.lagna);
    let tie = |x: f64| (top - x).abs() <= 1e-9 * top.abs().max(1.0);
    let mut out = Vec::new();
    if tie(s.lagna) {
        out.push(AyurMethod::Amsayu);
    }
    if tie(s.sun) {
        out.push(AyurMethod::Pindayu);
    }
    if tie(s.moon) {
        out.push(AyurMethod::Nisargayu);
    }
    out
}

/// The reckoning the strengths select, averaged when selectors tie.
pub fn ayurdaya(input: &AyurInput, strengths: &Strengths) -> Ayurdaya {
    let methods = select_methods(strengths);
    let selection = format!(
        "strengths: lagna {:.3}, Sun {:.3}, Moon {:.3}; the strongest selects {}",
        strengths.lagna,
        strengths.sun,
        strengths.moon,
        methods
            .iter()
            .map(|m| m.name())
            .collect::<Vec<_>>()
            .join(" and ")
    );
    if methods.len() == 1 {
        let mut a = one_method(input, methods[0]);
        a.selection = selection.clone();
        a.words.insert(1, selection);
        a
    } else {
        average(
            methods.iter().map(|m| one_method(input, *m)).collect(),
            format!("{selection}; equally strong selectors, so the methods are averaged"),
        )
    }
}

/// The reckoning with the selectors' strengths taken from Shadbala: the Sun's and the
/// Moon's ratio of total to required strength, and the lagna's Bhava bala against its
/// seven-rupa requirement (`kshana_bala`). Convention: the text says only "the
/// strongest"; these ratios put the three on one scale.
pub fn ayurdaya_with_shadbala(
    spk: &Spk,
    eop: Option<&EopTable>,
    obs: &Observer,
    chart: &Chart,
    ayanamsa: Ayanamsa,
) -> Result<Ayurdaya, MargaError> {
    let input = AyurInput::from_chart(chart)?;
    let s = kshana_bala::shadbala(spk, eop, obs, chart, ayanamsa)?;
    let ratio = |g: DashaLord| {
        s.grahas
            .iter()
            .find(|b| b.graha == g)
            .map(|b| b.ratio)
            .unwrap_or(0.0)
    };
    let strengths = Strengths {
        sun: ratio(DashaLord::Sun),
        moon: ratio(DashaLord::Moon),
        lagna: s.bhavas.first().map(|b| b.ratio).unwrap_or(0.0),
    };
    Ok(ayurdaya(&input, &strengths))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::{kernel, kochi_chart, KOCHI};

    fn planet(g: DashaLord, lon: f64) -> AyurPlanet {
        AyurPlanet {
            graha: g,
            sidereal: lon,
            retrograde: false,
            benefic: matches!(
                g,
                DashaLord::Jupiter | DashaLord::Venus | DashaLord::Mercury
            ),
            combust: false,
        }
    }

    /// Every planet at deep exaltation, lagna 0° Aries, nothing in the visible half.
    fn exalted() -> AyurInput {
        AyurInput {
            lagna: 0.0,
            planets: vec![
                planet(DashaLord::Sun, 10.0),
                planet(DashaLord::Moon, 33.0),
                planet(DashaLord::Mars, 298.0),
                planet(DashaLord::Mercury, 165.0),
                planet(DashaLord::Jupiter, 95.0),
                planet(DashaLord::Venus, 357.0),
                planet(DashaLord::Saturn, 200.0),
            ],
            benefic_on_lagna: false,
        }
    }

    #[test]
    fn planetary_years_tables() {
        let p: f64 = SEVEN.iter().map(|g| pindayu_years(*g)).sum();
        assert_eq!(p, 127.0);
        let n: f64 = SEVEN.iter().map(|g| nisargayu_years(*g)).sum();
        assert_eq!(n, 120.0);
        assert_eq!(pindayu_years(DashaLord::Venus), 21.0);
        assert_eq!(nisargayu_years(DashaLord::Saturn), 50.0);
        assert_eq!(pindayu_years(DashaLord::Rahu), 0.0);
        assert_eq!(exaltation_arc(DashaLord::Sun, 10.0), 180.0);
        assert_eq!(exaltation_arc(DashaLord::Sun, 190.0), 0.0);
        assert!((exaltation_arc(DashaLord::Sun, 100.0) - 90.0).abs() < 1e-9);
        assert_eq!(proportioned_years(20.0, 180.0), 20.0);
        assert_eq!(proportioned_years(20.0, 0.0), 10.0);
        assert_eq!(proportioned_years(20.0, 90.0), 15.0);
    }

    #[test]
    fn exalted_chart_gives_full_terms_and_is_clamped() {
        let a = ayurdaya_by(&exalted(), AyurMethod::Pindayu);
        // Mars at 298° is in the 10th 30° arc from 0° Aries (malefic, a third), Venus at
        // 357° in the 12th (benefic, half); Saturn at 200° in the 7th (a sixth), Mercury
        // at 165° in the 6th (nothing), Jupiter 95° in the 4th, Moon 33° 2nd, Sun 1st.
        let find = |n: &str| a.contributions.iter().find(|c| c.graha == n).unwrap();
        assert!((find("Sun").base_years - 19.0).abs() < 1e-9);
        assert!(
            (find("Mars").years - 10.0).abs() < 1e-9,
            "{:?}",
            find("Mars")
        );
        assert!((find("Venus").years - 10.5).abs() < 1e-9);
        assert!((find("Saturn").years - 20.0 * 5.0 / 6.0).abs() < 1e-9);
        assert!((find("Jupiter").years - 15.0).abs() < 1e-9);
        assert!((find("Lagna").years).abs() < 1e-9);
        // Sun in the lagna sign: Krurodaya with the lagna at 0° removes nothing
        assert!(a.krurodaya.is_some());
        assert!(a.krurodaya.as_ref().unwrap().years.abs() < 1e-9);
        assert!((0.0..=PARAMAYU).contains(&a.years));
        assert!(a.words[0] == LONGEVITY_CAVEAT);
        assert!(a.words.iter().all(|w| !w.is_empty()));
        // Nisargayu of the same chart: 120 natural years at full term, less the haranas
        let n = ayurdaya_by(&exalted(), AyurMethod::Nisargayu);
        assert!(n.years < 120.0 && n.years > 90.0, "{}", n.years);
        assert_eq!(n.words[0], LONGEVITY_CAVEAT);
    }

    #[test]
    fn haranas_and_clamp() {
        // Everything in the 12th as malefics: all lost
        let mut i = exalted();
        i.lagna = 0.0;
        for p in i.planets.iter_mut() {
            p.sidereal = 345.0;
            p.benefic = false;
        }
        let a = ayurdaya_by(&i, AyurMethod::Pindayu);
        // only the strongest of the seven loses (Venus, 345° is 12° from its deep
        // exaltation at 357°); the others keep their years
        let lost: Vec<&Contribution> = a
            .contributions
            .iter()
            .filter(|c| c.graha != "Lagna" && c.years.abs() < 1e-9)
            .collect();
        assert_eq!(lost.len(), 1, "{:?}", a.contributions);
        assert_eq!(lost[0].graha, "Venus");
        // combustion halves, Venus and Saturn excepted
        let mut i = exalted();
        for p in i.planets.iter_mut() {
            p.combust = p.graha != DashaLord::Sun;
        }
        let a = ayurdaya_by(&i, AyurMethod::Pindayu);
        let find = |n: &str| a.contributions.iter().find(|c| c.graha == n).unwrap();
        assert!((find("Jupiter").years - 7.5).abs() < 1e-9);
        assert!((find("Venus").years - 10.5).abs() < 1e-9); // chakrapata only
        assert!(find("Venus")
            .reductions
            .iter()
            .any(|r| r.name == "Astangata harana" && r.years == 0.0));
        // enemy sign: the Sun in Aquarius (Saturn's) loses a third; 120° from its
        // debilitation it holds 19 × 300 / 360 years
        let mut i = exalted();
        i.planets[0].sidereal = 310.0;
        i.lagna = 250.0; // the Sun is then in the 3rd arc: no Chakrapata
        let a = ayurdaya_by(&i, AyurMethod::Pindayu);
        let sun = a.contributions.iter().find(|c| c.graha == "Sun").unwrap();
        assert!((sun.base_years - 19.0 * 300.0 / 360.0).abs() < 1e-9);
        assert!(
            (sun.years - sun.base_years * 2.0 / 3.0).abs() < 1e-9,
            "{sun:?}"
        );
        // the Paramayu clamp: the exalted chart with the lagna at 29° Pisces adds
        // nearly twelve lagna years and passes 120
        let mut i = exalted();
        i.lagna = 359.0;
        let a = ayurdaya_by(&i, AyurMethod::Pindayu);
        assert!(a.raw_years > PARAMAYU, "{}", a.raw_years);
        assert_eq!(a.years, PARAMAYU);
        assert!(a.words.iter().any(|w| w.contains("Paramayu")));
    }

    #[test]
    fn amsayu_counts_navamsas_and_applies_bharanas() {
        // 0° Aries: 0 navamsas → 0 years; 10° Taurus = 2400′ → 12 navamsas → remainder 0;
        // 3°20′ Aries = 1 navamsa → 1 year
        assert!(navamsa_years(0.0).abs() < 1e-9);
        assert!(navamsa_years(40.0).abs() < 1e-9);
        assert!((navamsa_years(10.0 / 3.0) - 1.0).abs() < 1e-9);
        assert!((navamsa_years(100.0) - 6.0).abs() < 1e-9); // 30 navamsas → 6
        let mut i = exalted();
        i.lagna = 100.0;
        for p in i.planets.iter_mut() {
            p.sidereal = 100.0 + 20.0; // 120° = 36 navamsas → 0 years before bharana
        }
        i.planets[0].sidereal = 10.0; // Sun exalted: 3 navamsas → 3 years, trebled
        let a = ayurdaya_by(&i, AyurMethod::Amsayu);
        let sun = a.contributions.iter().find(|c| c.graha == "Sun").unwrap();
        assert!((sun.base_years - 3.0).abs() < 1e-9);
        assert!(sun
            .reductions
            .iter()
            .any(|r| r.name == "Bharana" && r.factor == 3.0));
        assert!(a.krurodaya.is_none());
        let lagna = a.contributions.iter().find(|c| c.graha == "Lagna").unwrap();
        assert!((lagna.years - 6.0).abs() < 1e-9);
        assert_eq!(a.words[0], LONGEVITY_CAVEAT);
    }

    #[test]
    fn method_selection() {
        let s = Strengths {
            sun: 1.2,
            moon: 0.9,
            lagna: 1.0,
        };
        assert_eq!(select_methods(&s), vec![AyurMethod::Pindayu]);
        let s = Strengths {
            sun: 1.0,
            moon: 1.0,
            lagna: 0.5,
        };
        assert_eq!(
            select_methods(&s),
            vec![AyurMethod::Pindayu, AyurMethod::Nisargayu]
        );
        let a = ayurdaya(&exalted(), &s);
        assert_eq!(a.method, AyurMethod::Average);
        assert_eq!(a.components.len(), 2);
        let mean = (a.components[0].years + a.components[1].years) / 2.0;
        assert!((a.years - mean).abs() < 1e-9);
        assert!(a.words.contains(&LONGEVITY_CAVEAT.to_string()));
        let s = Strengths {
            sun: 0.1,
            moon: 0.2,
            lagna: 0.3,
        };
        let a = ayurdaya(&exalted(), &s);
        assert_eq!(a.method, AyurMethod::Amsayu);
        assert!(a.selection.contains("Amsayu"));
        let all = ayurdaya_by(&exalted(), AyurMethod::Average);
        assert_eq!(all.components.len(), 3);
    }

    #[test]
    fn kochi_ayurdaya_from_the_kernel() {
        let Some(spk) = kernel() else { return };
        let ch = kochi_chart(&spk);
        let a = ayurdaya_with_shadbala(&spk, None, &KOCHI, &ch, Ayanamsa::Lahiri).unwrap();
        assert!((0.0..=PARAMAYU).contains(&a.years), "{}", a.years);
        assert_eq!(a.words[0], LONGEVITY_CAVEAT);
        assert!(!a.selection.is_empty());
        let input = AyurInput::from_chart(&ch).unwrap();
        assert_eq!(input.planets.len(), 7);
        for m in ayurdaya_all(&input) {
            assert!((0.0..=PARAMAYU).contains(&m.years), "{:?}", m.method);
            assert_eq!(m.contributions.len(), 8);
            assert!(m.words.iter().all(|w| !w.is_empty()));
            assert!(m
                .contributions
                .iter()
                .all(|c| !c.words.is_empty() && c.years >= 0.0));
        }
    }
}
