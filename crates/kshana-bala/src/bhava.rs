//! Bhava bala: the strength of the twelve houses (BPHS, Bhava bala slokas following
//! ch. 27; Raman, *Graha and Bhava Balas*, ch. on Bhava bala) in three parts:
//! Bhavadhipati bala, the Shadbala of the lord of the sign on the cusp; Bhava Dig bala,
//! by the class of the cusp's sign ([`crate::tables::bhava_dig_bala`]); and Bhava Drishti
//! bala, the aspects on the cusp (bhava madhya).

use crate::drik::{drishti_pinda, Aspector};
use crate::tables::{bhava_dig_bala, sputa_drishti, BHAVA_REQUIRED_VIRUPAS, RUPA};
use kshana_vedic::{sign_lord, DashaLord};

/// Strength of one house.
#[derive(Clone, Debug, PartialEq)]
pub struct BhavaBala {
    /// House 1–12.
    pub house: u8,
    /// Lord of the sign on the cusp.
    pub lord: DashaLord,
    /// Bhavadhipati bala: the lord's total Shadbala, virupas.
    pub lord_bala: f64,
    /// Bhava Dig bala, virupas.
    pub dig: f64,
    /// Bhava Drishti bala, virupas.
    pub drishti: f64,
    /// The three added, virupas.
    pub total: f64,
    /// The total in rupas.
    pub rupas: f64,
    /// The total against seven rupas.
    pub ratio: f64,
}

/// Bhava Drishti bala on a cusp: a quarter of the benefic aspects less a quarter of the
/// malefic ones, as for a graha, plus the full aspect of Jupiter and of Mercury when they
/// aspect the bhava (BPHS 27.19 in Santhanam: "if Jupiter or Mercury give a drishti to a
/// bhava, add that graha's drishti also"; JHora's port keeps their aspects whole where the
/// others are quartered). Raman's worked chart applies only the quartered net; the extra
/// term is the text's.
pub fn bhava_drishti_bala(cusp: f64, aspectors: &[Aspector]) -> f64 {
    let (b, m) = drishti_pinda(cusp, aspectors, None);
    let extra: f64 = aspectors
        .iter()
        .filter(|a| matches!(a.graha, DashaLord::Jupiter | DashaLord::Mercury))
        .map(|a| sputa_drishti(a.graha, (cusp - a.sidereal).rem_euclid(360.0)))
        .sum();
    (b - m) / 4.0 + extra
}

/// Bhava balas for the twelve sidereal cusps. `lord_bala` gives each graha's total
/// Shadbala in virupas.
pub fn bhava_balas(
    cusps_sidereal: &[f64; 12],
    lord_bala: impl Fn(DashaLord) -> f64,
    aspectors: &[Aspector],
) -> Vec<BhavaBala> {
    (0..12)
        .map(|i| {
            let cusp = cusps_sidereal[i].rem_euclid(360.0);
            let lord = sign_lord((cusp / 30.0).floor() as usize);
            let lb = lord_bala(lord);
            let dig = bhava_dig_bala(cusp, i);
            let drishti = bhava_drishti_bala(cusp, aspectors);
            let total = lb + dig + drishti;
            BhavaBala {
                house: (i + 1) as u8,
                lord,
                lord_bala: lb,
                dig,
                drishti,
                total,
                rupas: total / RUPA,
                ratio: total / BHAVA_REQUIRED_VIRUPAS,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn twelve_houses_with_their_lords() {
        let cusps: [f64; 12] = std::array::from_fn(|i| (10.0 + 30.0 * i as f64) % 360.0);
        let asp = [
            Aspector {
                graha: DashaLord::Jupiter,
                sidereal: 190.0,
                benefic: true,
            },
            Aspector {
                graha: DashaLord::Saturn,
                sidereal: 100.0,
                benefic: false,
            },
        ];
        let b = bhava_balas(
            &cusps,
            |g| if g == DashaLord::Mars { 400.0 } else { 300.0 },
            &asp,
        );
        assert_eq!(b.len(), 12);
        assert_eq!(b[0].lord, DashaLord::Mars);
        assert_eq!(b[0].lord_bala, 400.0);
        // Scorpio on the 8th
        assert_eq!(b[7].lord, DashaLord::Mars);
        // 10° Aries is a quadruped cusp in the lagna: 30
        assert_eq!(b[0].dig, 30.0);
        // Jupiter opposite the lagna: full aspect 60 quartered (15) plus the whole 60
        let d1 = bhava_drishti_bala(10.0, &asp[..1]);
        assert!((d1 - 75.0).abs() < 1e-12, "{d1}");
        for h in &b {
            assert!((h.total - (h.lord_bala + h.dig + h.drishti)).abs() < 1e-9);
            assert!((h.rupas - h.total / 60.0).abs() < 1e-12);
        }
    }
}
