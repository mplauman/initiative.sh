mod beach;

use initiative_macros::WordList;
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::world::{place::PlaceType, Demographics, Place};

use super::LocationType;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, WordList)]
#[serde(into = "&'static str", try_from = "&str")]
pub enum GeographicalType {
    Beach,
    #[alias = "gorge"]
    Canyon,
    #[alias = "cavern"]
    Cave,
    Chasm,
    Glacier,
    Grove,
    Hill,
    Island,
    Monolith,
    Oasis,
    Pass,
    Peninsula,
    Ridge,
    Rift,
    River,
    Tree,
    #[alias = "vale"]
    Valley,
}

impl GeographicalType {
    pub const fn get_emoji(&self) -> Option<&'static str> {
        match self {
            Self::Beach => Some("🏖"),
            Self::Canyon | Self::Chasm | Self::River | Self::Valley => Some("🏞"),
            Self::Glacier => Some("🏔"),
            Self::Grove | Self::Tree => Some("🌳"),
            Self::Hill | Self::Pass | Self::Ridge => Some("⛰"),
            Self::Island | Self::Peninsula => Some("🏝"),
            Self::Monolith => Some("🗿"),
            Self::Oasis => Some("🌴"),
            Self::Cave | Self::Rift => None,
        }
    }
}

pub fn generate(place: &mut Place, rng: &mut impl Rng, demographics: &Demographics) {
    #[allow(clippy::collapsible_match)]
    if let Some(PlaceType::Location(LocationType::Geographical(subtype))) = place.subtype.value() {
        #[allow(clippy::single_match)]
        match subtype {
            GeographicalType::Beach => beach::generate(place, rng, demographics),
            _ => {}
        }
    }
}
