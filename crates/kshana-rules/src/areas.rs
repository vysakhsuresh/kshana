//! Life areas as house groups. The groups follow standard KP practice; the primary
//! cusp is the one whose sub-lord decides the promise.

/// A life area.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Area {
    /// Name.
    pub name: &'static str,
    /// The cusp whose sub-lord is judged.
    pub primary: u8,
    /// Houses whose signification fulfils the matter.
    pub group: &'static [u8],
    /// Houses whose signification denies or delays it (the 12th from each group house).
    pub negating: &'static [u8],
    /// Divisional chart used for strength (informational for now).
    pub varga: &'static str,
}

/// The nine areas the app offers.
pub const AREAS: [Area; 9] = [
    Area {
        name: "Career",
        primary: 10,
        group: &[2, 6, 10, 11],
        negating: &[1, 5, 9],
        varga: "D10",
    },
    Area {
        name: "Marriage",
        primary: 7,
        group: &[2, 7, 11],
        negating: &[1, 6, 10],
        varga: "D9",
    },
    Area {
        name: "Children",
        primary: 5,
        group: &[2, 5, 11],
        negating: &[1, 4, 10],
        varga: "D7",
    },
    Area {
        name: "Wealth",
        primary: 11,
        group: &[2, 6, 11],
        negating: &[1, 5, 10, 12],
        varga: "D2",
    },
    Area {
        name: "Health",
        primary: 1,
        group: &[1, 5, 11],
        negating: &[6, 8, 12],
        varga: "D30",
    },
    Area {
        name: "Property",
        primary: 4,
        group: &[4, 11, 12],
        negating: &[3, 10],
        varga: "D4",
    },
    Area {
        name: "Education",
        primary: 4,
        group: &[4, 9, 11],
        negating: &[3, 8, 10],
        varga: "D24",
    },
    Area {
        name: "Family",
        primary: 2,
        group: &[2, 4, 11],
        negating: &[1, 3, 10],
        varga: "D4",
    },
    Area {
        name: "Foreign",
        primary: 12,
        group: &[3, 9, 12],
        negating: &[4],
        varga: "D9",
    },
];

/// Look an area up by name.
pub fn area(name: &str) -> Option<Area> {
    AREAS
        .iter()
        .copied()
        .find(|a| a.name.eq_ignore_ascii_case(name))
}
