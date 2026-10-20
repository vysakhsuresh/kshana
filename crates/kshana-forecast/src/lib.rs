//! Forecasts for one person: what the sky is doing to their chart now and next.
//!
//! Everything here is a function of a natal [`Chart`] and an instant. Transits are
//! solved to exact instants with the same root finder as everything else; the day's
//! hours come from the solar day, the horas and the person's Moon; the life map lays
//! the Vimshottari periods over the years with the Parashari quality of each lord for
//! each area, so a person can see the shape of a life at a glance.

#![forbid(unsafe_code)]

pub mod calendar;
pub mod daily;
pub mod gochara;
pub mod lifemap;
pub mod transits;

pub use calendar::{observances, sidereal_sign_name, Observance};
pub use daily::{daily, DailyForecast, HourWindow};
pub use gochara::{
    gochara_longitude, gochara_verdict, kakshya_favourable, kakshya_of, kakshya_windows, moorthi,
    score_blend, today_transit_score, transit_bindus, transit_windows, vedha_exempt, vedha_table,
    GocharaVerdict, KakshyaLord, KakshyaWindow, Moorthi, TransitWindow, GOCHARA_PLANETS,
    KAKSHYA_DEG, KAKSHYA_LORDS,
};
pub use lifemap::{life_map, YearRow};
pub use transits::{sade_sati, transits_at, upcoming_transits, SadeSati, Transit, TransitKind};

#[cfg(test)]
mod tests {
    use super::*;
    use kshana_chart::{compute, ChartInput};
    use kshana_ephem::Spk;
    use kshana_houses::HouseSystem;
    use kshana_sky::Observer;
    use kshana_time::Utc;
    use kshana_vedic::Ayanamsa;

    fn fixture() -> Option<(Spk, kshana_chart::Chart, Observer, kshana_time::Instant)> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        let spk = Spk::open(&p).ok()?;
        let obs = Observer {
            latitude_deg: 9.9312,
            longitude_deg: 76.2673,
            height_m: 0.0,
        };
        let birth = Utc::new(1990, 10, 29, 17, 36, 0, 0)
            .unwrap()
            .to_instant()
            .unwrap();
        let ch = compute(
            &spk,
            None,
            &ChartInput {
                instant: birth,
                observer: obs,
                system: HouseSystem::Placidus,
                ayanamsa: Ayanamsa::Lahiri,
                topocentric_moon: false,
            },
        )
        .unwrap();
        let at = Utc::new(2026, 10, 15, 9, 0, 0, 0)
            .unwrap()
            .to_instant()
            .unwrap();
        Some((spk, ch, obs, at))
    }

    #[test]
    fn transits_and_sade_sati_are_consistent() {
        let Some((spk, ch, _, at)) = fixture() else {
            return;
        };
        let ts = transits_at(&spk, &ch, at, Ayanamsa::Lahiri).unwrap();
        assert!(ts
            .iter()
            .any(|t| t.kind == TransitKind::FromMoon && t.body == "Saturn"));
        for t in &ts {
            assert!((1..=12).contains(&t.house));
        }
        let ss = sade_sati(&spk, &ch, at, Ayanamsa::Lahiri).unwrap();
        let sat = ts
            .iter()
            .find(|t| t.kind == TransitKind::FromMoon && t.body == "Saturn")
            .unwrap();
        let expect = match sat.house {
            12 => 1,
            1 => 2,
            2 => 3,
            _ => 0,
        };
        assert_eq!(ss.phase, expect);
        if ss.phase == 0 {
            assert!(ss.next_start.is_some());
        } else {
            assert!(ss.phase_end.is_some());
        }
        let up = upcoming_transits(&spk, Ayanamsa::Lahiri, at, 400).unwrap();
        assert!(up.iter().any(|u| u.1 == "Sun"));
        assert!(up.windows(2).all(|w| w[0].0 <= w[1].0));
    }

    #[test]
    fn daily_has_windows_and_a_headline() {
        let Some((spk, ch, obs, at)) = fixture() else {
            return;
        };
        let d = daily(&spk, None, &obs, &ch, at, Ayanamsa::Lahiri).unwrap();
        assert!(d.day.is_some());
        assert!(d.windows.iter().any(|w| w.avoid));
        assert!(d.windows.iter().any(|w| !w.avoid));
        assert!(d.windows.windows(2).all(|w| w[0].start <= w[1].start));
        assert!(!d.headline.is_empty());
        assert!((1..=12).contains(&d.chandra_bala.0));
    }

    #[test]
    fn life_map_covers_the_years() {
        let Some((_, ch, _, _)) = fixture() else {
            return;
        };
        let rows = life_map(&ch, 80);
        assert_eq!(rows.len(), 81);
        assert!(rows
            .iter()
            .all(|r| r.areas.iter().all(|q| (0.0..=1.0).contains(q))));
        assert!(rows.windows(2).all(|w| w[0].start < w[1].start));
    }

    #[test]
    fn calendar_lists_the_month() {
        let Some((spk, _, obs, at)) = fixture() else {
            return;
        };
        let o = observances(&spk, None, &obs, Ayanamsa::Lahiri, at, 35).unwrap();
        let names: Vec<_> = o.iter().map(|x| x.name).collect();
        assert!(names.contains(&"Ekadashi (Shukla)"));
        assert!(names.contains(&"Ekadashi (Krishna)"));
        assert!(names.contains(&"New Moon (exact)"));
        assert!(names.contains(&"Full Moon (exact)"));
        assert!(names.iter().any(|n| n.ends_with("Sankranti")));
        assert!(o.windows(2).all(|w| w[0].exact <= w[1].exact));
        assert!(o
            .iter()
            .filter(|x| x.name.starts_with("Ekadashi"))
            .all(|x| x.observed_sunrise.is_some()));
    }
}
