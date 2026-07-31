//! Burgs / settlements (slot `[15]`: `pack.burgs` JSON `[{}, {"cell":1133,"x":1468.66,...}]`).
//!
//! Slot `[0]` of `pack.burgs` is always `{}` (placeholder). In Voronia we
//! initialize it as `Burg::placeholder()` to keep ids 1-based.

use super::coat_of_arms::CoatOfArms;
use super::culture::CultureType;

/// A burg / settlement.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Burg {
    /// Id (its index in `pack.burgs`; 0 = placeholder).
    pub id: u16,
    /// Burg name (e.g. "Tal").
    pub name: String,
    /// Pack cell where it is located.
    pub cell: u32,
    /// Coordinates `[x, y]` in canvas units (not cell center; may be the exact point).
    pub position: [f32; 2],
    /// Culture id.
    pub culture: u16,
    /// State id.
    pub state: u16,
    /// Id of the feature (island/lake/ocean) it falls in.
    pub feature: u32,
    /// Population in "points" (`f32`; 1 pt = 1000 people by default).
    #[serde(default)]
    pub population: f32,
    /// Burg's culture type (same enum as `Culture`).
    #[serde(default)]
    pub kind: CultureType,
    /// Coat of arms (Armoria-compatible).
    #[serde(default)]
    pub coat_of_arms: CoatOfArms,
    /// `true` if it is the state capital.
    #[serde(default)]
    pub is_capital: bool,
    /// Id of a water feature with a harbor (`Some` if it is a harbor; harbor cell via `haven_cell`).
    #[serde(default)]
    pub port_feature: Option<u32>,
    /// MFCG flags (Watabou — Medieval Fantasy City Generator). Important to keep them
    /// for re-export compatibility, even though Voronia v1 does not integrate MFCG.
    #[serde(default)]
    pub has_citadel: bool,
    #[serde(default)]
    pub has_plaza: bool,
    #[serde(default)]
    pub has_shanty: bool,
    #[serde(default)]
    pub has_temple: bool,
    #[serde(default)]
    pub has_walls: bool,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub removed: bool,
}

impl Burg {
    /// Reserved placeholder — mirrors the (empty) slot `[0]` that Azgaar leaves in `pack.burgs`.
    #[inline]
    pub fn placeholder() -> Self {
        Self {
            id: 0,
            name: String::new(),
            cell: 0,
            position: [0.0, 0.0],
            culture: 0,
            state: 0,
            feature: 0,
            population: 0.0,
            kind: CultureType::Generic,
            coat_of_arms: CoatOfArms::default(),
            is_capital: false,
            port_feature: None,
            has_citadel: false,
            has_plaza: false,
            has_shanty: false,
            has_temple: false,
            has_walls: false,
            locked: false,
            removed: false,
        }
    }
}
