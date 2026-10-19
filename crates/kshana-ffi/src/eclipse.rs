//! Eclipses over the FFI.

use kshana_eclipse::ShadowRule;
use kshana_sky::Observer;

use crate::{Engine, ExactInstant, KshanaError};

/// One eclipse, solar or lunar, with its contacts.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct EclipseRecord {
    /// "solar" or "lunar".
    pub family: String,
    /// Partial, Annular, Total, Hybrid; Penumbral, Partial, Total.
    pub kind: String,
    pub greatest: ExactInstant,
    pub begins: Option<ExactInstant>,
    pub ends: Option<ExactInstant>,
    pub gamma: f64,
    pub magnitude: f64,
    pub penumbral_magnitude: f64,
    pub saros: Option<i32>,
    /// Lunar contacts P1, U1, U2, U3, U4, P4 when they exist.
    pub contacts: Vec<Option<ExactInstant>>,
    /// Local circumstances for solar eclipses: C1, C2, max, C3, C4 when visible.
    pub local_c1: Option<ExactInstant>,
    pub local_c2: Option<ExactInstant>,
    pub local_max: Option<ExactInstant>,
    pub local_c3: Option<ExactInstant>,
    pub local_c4: Option<ExactInstant>,
    pub local_magnitude: f64,
    pub local_obscuration: f64,
    pub visible_here: bool,
}

/// The lunation before birth.
#[derive(Clone, Debug, uniffi::Record)]
#[allow(missing_docs)]
pub struct SyzygyRecord {
    pub at: ExactInstant,
    pub kind: String,
    pub longitude: f64,
    pub prenatal_eclipse: Option<EclipseRecord>,
}

fn err<E: std::fmt::Debug>(e: E) -> KshanaError {
    KshanaError::Engine(format!("{e:?}"))
}

fn solar_record(
    engine: &Engine,
    e: &kshana_eclipse::SolarEclipse,
    obs: Option<&Observer>,
) -> EclipseRecord {
    let local = obs.and_then(|o| {
        kshana_eclipse::local_solar(&engine.spk, engine.eop.as_ref(), o, e)
            .ok()
            .flatten()
    });
    EclipseRecord {
        family: "solar".into(),
        kind: format!("{:?}", e.kind),
        greatest: e.greatest.into(),
        begins: Some(e.begins.into()),
        ends: Some(e.ends.into()),
        gamma: e.gamma,
        magnitude: e.magnitude_max,
        penumbral_magnitude: 0.0,
        saros: e.saros,
        contacts: Vec::new(),
        local_c1: local.as_ref().map(|l| l.c1.into()),
        local_c2: local.as_ref().and_then(|l| l.c2.map(Into::into)),
        local_max: local.as_ref().map(|l| l.max.into()),
        local_c3: local.as_ref().and_then(|l| l.c3.map(Into::into)),
        local_c4: local.as_ref().map(|l| l.c4.into()),
        local_magnitude: local.as_ref().map(|l| l.magnitude).unwrap_or(0.0),
        local_obscuration: local.as_ref().map(|l| l.obscuration).unwrap_or(0.0),
        visible_here: local.as_ref().map(|l| l.visible).unwrap_or(false),
    }
}

fn lunar_record(
    engine: &Engine,
    e: &kshana_eclipse::LunarEclipse,
    obs: Option<&Observer>,
) -> EclipseRecord {
    let visible = obs
        .map(|o| {
            e.visible_at(&engine.spk, engine.eop.as_ref(), o)
                .unwrap_or(false)
        })
        .unwrap_or(false);
    EclipseRecord {
        family: "lunar".into(),
        kind: format!("{:?}", e.kind),
        greatest: e.greatest.into(),
        begins: e.contacts[0].map(Into::into),
        ends: e.contacts[5].map(Into::into),
        gamma: e.gamma,
        magnitude: e.umb_magnitude,
        penumbral_magnitude: e.pen_magnitude,
        saros: e.saros,
        contacts: e.contacts.iter().map(|c| c.map(Into::into)).collect(),
        local_c1: None,
        local_c2: None,
        local_max: None,
        local_c3: None,
        local_c4: None,
        local_magnitude: 0.0,
        local_obscuration: 0.0,
        visible_here: visible,
    }
}

#[uniffi::export]
impl Engine {
    /// Every solar and lunar eclipse in `[from, to]` (at most 20 years), with local
    /// circumstances at a place.
    pub fn eclipses(
        &self,
        from: ExactInstant,
        to: ExactInstant,
        latitude_deg: f64,
        longitude_deg: f64,
    ) -> Result<Vec<EclipseRecord>, KshanaError> {
        let f: kshana_time::Instant = from.into();
        let t: kshana_time::Instant = to.into();
        if t.ns_since(f) > 20 * 366 * 86_400 * 1_000_000_000 {
            return Err(KshanaError::Engine(
                "eclipse searches cover at most 20 years per call".into(),
            ));
        }
        let o = Observer {
            latitude_deg,
            longitude_deg,
            height_m: 0.0,
        };
        let mut out: Vec<EclipseRecord> = kshana_eclipse::solar_eclipses(&self.spk, f, t)
            .map_err(err)?
            .iter()
            .map(|e| solar_record(self, e, Some(&o)))
            .collect();
        out.extend(
            kshana_eclipse::lunar_eclipses(&self.spk, f, t, ShadowRule::Danjon)
                .map_err(err)?
                .iter()
                .map(|e| lunar_record(self, e, Some(&o))),
        );
        out.sort_by_key(|e| (e.greatest.tai_seconds, e.greatest.nanos));
        Ok(out)
    }

    /// The lunation before birth, and the last eclipse before it.
    pub fn prenatal(&self, birth: ExactInstant) -> Result<SyzygyRecord, KshanaError> {
        let s = kshana_eclipse::prenatal_syzygy(&self.spk, birth.into()).map_err(err)?;
        let pe = kshana_eclipse::prenatal_eclipse(&self.spk, birth.into(), ShadowRule::Danjon)
            .ok()
            .map(|e| match e {
                kshana_eclipse::PrenatalEclipse::Solar(x) => solar_record(self, &x, None),
                kshana_eclipse::PrenatalEclipse::Lunar(x) => lunar_record(self, &x, None),
            });
        Ok(SyzygyRecord {
            at: s.at.into(),
            kind: format!("{:?}", s.kind),
            longitude: s.longitude,
            prenatal_eclipse: pe,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{instant_from_utc, UtcFields};

    #[test]
    fn eclipses_cross_the_boundary() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/de421.bsp");
        if !p.exists() {
            return;
        }
        let engine = Engine::open(p.to_string_lossy().into_owned(), None).unwrap();
        let u = |y, mo, d| {
            instant_from_utc(UtcFields {
                year: y,
                month: mo,
                day: d,
                hour: 0,
                minute: 0,
                second: 0,
                nanos: 0,
            })
            .unwrap()
        };
        let e = engine
            .eclipses(u(2024, 1, 1), u(2025, 1, 1), 9.9312, 76.2673)
            .unwrap();
        assert!(e.iter().any(|x| x.family == "solar" && x.kind == "Total"));
        assert!(e.iter().any(|x| x.family == "lunar"));
        let s = engine.prenatal(u(1990, 10, 29)).unwrap();
        assert!(s.at.tai_seconds < u(1990, 10, 29).tai_seconds);
    }
}
