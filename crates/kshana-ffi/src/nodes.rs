//! The lunar node the whole engine uses for Rahu and Ketu, and the ayanamsa catalogue.
//!
//! One process-wide node choice ([`kshana_sky::set_lunar_node`]) feeds the natal chart's
//! Rahu (`Chart::rahu`), transits, gochara, trace and parihara alike, so natal and
//! transit Rahu are always the same point. Default: the true (osculating) node, which
//! the natal chart has always used; "mean" keeps the smooth mean node most printed
//! panchangas use for Rahu's sign changes.

use kshana_sky::LunarNode;

use crate::AyanamsaChoice;

/// Set the node used for Rahu and Ketu everywhere: "true" (the default, the osculating
/// node) or "mean". Returns false, changing nothing, for any other name.
#[uniffi::export]
pub fn set_lunar_node(name: String) -> bool {
    match name.to_ascii_lowercase().as_str() {
        "true" | "osculating" => {
            kshana_sky::set_lunar_node(LunarNode::True);
            true
        }
        "mean" => {
            kshana_sky::set_lunar_node(LunarNode::Mean);
            true
        }
        _ => false,
    }
}

/// The node in use for Rahu and Ketu: "true" or "mean".
#[uniffi::export]
pub fn lunar_node() -> String {
    match kshana_sky::lunar_node() {
        LunarNode::True => "true".into(),
        LunarNode::Mean => "mean".into(),
    }
}

/// The Swiss Ephemeris `SE_SIDM_*` number an ayanamsa choice reproduces (for the
/// provenance line and for checking against other software).
#[uniffi::export]
pub fn ayanamsa_swiss_ephemeris_mode(choice: AyanamsaChoice) -> i32 {
    kshana_vedic::Ayanamsa::from(choice).swiss_ephemeris_mode()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_names_round_trip_and_bad_names_change_nothing() {
        // The engine-wide default is the true node; other tests in this binary rely on
        // it, so this test only ever leaves it set to "true".
        assert_eq!(lunar_node(), "true");
        assert!(!set_lunar_node("apparent".into()));
        assert_eq!(lunar_node(), "true");
        assert!(set_lunar_node("TRUE".into()));
        assert_eq!(lunar_node(), "true");
    }

    #[test]
    fn every_choice_maps_to_its_swiss_ephemeris_mode() {
        use AyanamsaChoice::*;
        let want = [
            (Lahiri, 1),
            (Raman, 3),
            (Krishnamurti, 5),
            (FaganBradley, 0),
            (TrueChitra, 27),
            (TrueRevati, 28),
            (TruePushya, 29),
            (Yukteshwar, 7),
            (SuryaSiddhanta, 21),
            (Lahiri1940, 43),
            (LahiriVp285, 44),
            (LahiriIcrc, 46),
            (KrishnamurtiVp291, 45),
        ];
        for (c, m) in want {
            assert_eq!(ayanamsa_swiss_ephemeris_mode(c), m, "{c:?}");
        }
    }
}
