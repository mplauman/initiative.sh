pub use building::*;

mod building;
mod view;

use super::region::Uuid as RegionUuid;
use super::{Demographics, Field, Generate};
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use view::{DescriptionView, DetailsView, SummaryView};

initiative_macros::uuid!();

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Location {
    pub uuid: Option<Uuid>,
    pub parent_uuid: Option<RegionUuid>,
    pub subtype: Field<LocationType>,

    pub name: Field<String>,
    pub description: Field<String>,
    // pub architecture: Option<String>,
    // pub floors: Field<u8>,
    // pub owner: Field<Vec<NpcUuid>>,
    // pub staff: Field<Vec<NpcUuid>>,
    // pub occupants: Field<Vec<NpcUuid>>,
    // pub services: Option<String>,
    // pub worship: Field<String>,
    // pub quality: something
    // pub price: something
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "subtype")]
pub enum LocationType {
    Building(Option<BuildingType>),
}

impl Location {
    pub fn display_summary(&self) -> SummaryView {
        SummaryView::new(self)
    }

    pub fn display_description(&self) -> DescriptionView {
        DescriptionView::new(self)
    }

    pub fn display_details(&self) -> DetailsView {
        DetailsView::new(self)
    }
}

impl Generate for Location {
    fn regenerate(&mut self, rng: &mut impl Rng, demographics: &Demographics) {
        self.subtype.replace_with(|location_type| {
            if let Some(mut location_type) = location_type {
                location_type.regenerate(rng, demographics);
                location_type
            } else {
                LocationType::generate(rng, demographics)
            }
        });

        if let Some(value) = self.subtype.value_mut() {
            match value {
                LocationType::Building(mut building_type) => {
                    if building_type.is_none() {
                        building_type.replace(BuildingType::generate(rng, demographics));
                        self.subtype = Field::Locked(LocationType::Building(building_type));
                    }

                    match building_type.unwrap() {
                        BuildingType::Inn => generate_inn(self, rng, demographics),
                    }
                }
            }
        }
    }
}

impl Default for LocationType {
    fn default() -> Self {
        Self::Building(Default::default())
    }
}

impl Generate for LocationType {
    fn regenerate(&mut self, rng: &mut impl Rng, demographics: &Demographics) {
        *self = Self::Building(Some(BuildingType::generate(rng, demographics)));
    }
}

impl fmt::Display for LocationType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Building(Some(building_type)) => write!(f, "{}", building_type),
            Self::Building(None) => write!(f, "building"),
        }
    }
}

impl FromStr for LocationType {
    type Err = ();

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        if let Ok(building_type) = raw.parse() {
            Ok(LocationType::Building(Some(building_type)))
        } else if raw == "building" {
            Ok(LocationType::Building(None))
        } else {
            Err(())
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn generate_test() {
        let demographics = Demographics::default();

        // This should fail when we start re-adding location types.
        let mut rng = SmallRng::seed_from_u64(0);
        assert_eq!(
            Location::generate(&mut rng, &demographics).subtype,
            Location::generate(&mut rng, &demographics).subtype,
        );

        let mut rng1 = SmallRng::seed_from_u64(0);
        let mut rng2 = SmallRng::seed_from_u64(0);
        assert_eq!(
            Location::generate(&mut rng1, &demographics).subtype,
            Location::generate(&mut rng2, &demographics).subtype,
        );
    }

    #[test]
    fn default_test() {
        assert_eq!(LocationType::Building(None), LocationType::default());
    }

    #[test]
    fn display_test() {
        assert_eq!(
            format!("{}", BuildingType::Inn),
            format!("{}", LocationType::Building(Some(BuildingType::Inn))),
        );

        assert_eq!("building", format!("{}", LocationType::Building(None)));
    }

    #[test]
    fn try_from_noun_test() {
        assert_eq!(
            Ok(LocationType::Building(Some(BuildingType::Inn))),
            "inn".parse(),
        );

        assert_eq!(Ok(LocationType::Building(None)), "building".parse());

        let location_type: Result<LocationType, ()> = "npc".parse();
        assert_eq!(Err(()), location_type);
    }

    #[test]
    fn location_type_serialize_deserialize_test() {
        assert_eq!(
            r#"{"type":"Building","subtype":null}"#,
            serde_json::to_string(&LocationType::Building(None)).unwrap(),
        );

        assert_eq!(
            r#"{"type":"Building","subtype":"Inn"}"#,
            serde_json::to_string(&LocationType::Building(Some(BuildingType::Inn))).unwrap(),
        );
    }

    #[test]
    fn location_serialize_deserialize_test() {
        let location = Location {
            uuid: Some(uuid::Uuid::nil().into()),
            parent_uuid: Some(uuid::Uuid::nil().into()),
            subtype: LocationType::Building(Some(BuildingType::Inn)).into(),

            name: "Oaken Mermaid Inn".into(),
            description: "I am Mordenkainen".into(),
        };

        assert_eq!(
            r#"{"uuid":"00000000-0000-0000-0000-000000000000","parent_uuid":"00000000-0000-0000-0000-000000000000","subtype":{"type":"Building","subtype":"Inn"},"name":"Oaken Mermaid Inn","description":"I am Mordenkainen"}"#,
            serde_json::to_string(&location).unwrap(),
        );

        let value: Location = serde_json::from_str(r#"{"uuid":"00000000-0000-0000-0000-000000000000","parent_uuid":"00000000-0000-0000-0000-000000000000","subtype":{"type":"Building","subtype":"Inn"},"name":"Oaken Mermaid Inn","description":"I am Mordenkainen"}"#).unwrap();

        assert_eq!(location, value);
    }
}
