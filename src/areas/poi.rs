use super::{Feature, Pos};
use dagr_lib::{
    content::ContentKey,
    world::{LocationId, LocationKind},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PoiKind {
    Ruins,
    Cave,
    Tomb,
    Lair,
    NaturalResource,
    Settlement,
    Mine,
    Dungeon,
}

impl PoiKind {
    pub fn from_content_key(key: &ContentKey) -> Self {
        match key.as_str().rsplit(':').next() {
            Some("ruins") => Self::Ruins,
            Some("caves") => Self::Cave,
            Some("tomb") => Self::Tomb,
            Some("lair") => Self::Lair,
            Some("natural_resource") => Self::NaturalResource,
            Some("settlement") => Self::Settlement,
            Some("mine") => Self::Mine,
            Some("dungeon") => Self::Dungeon,
            _ => Self::Ruins,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Ruins => "Ancient ruins",
            Self::Cave => "Cave entrance",
            Self::Tomb => "Forgotten tomb",
            Self::Lair => "Creature lair",
            Self::NaturalResource => "Natural resource",
            Self::Settlement => "Settlement",
            Self::Mine => "Mine workings",
            Self::Dungeon => "Dungeon entrance",
        }
    }

    pub fn feature(self) -> Feature {
        match self {
            Self::Ruins => Feature::RUINS,
            Self::Cave => Feature::CAVE,
            Self::Tomb => Feature::TOMB,
            Self::Lair => Feature::LAIR,
            Self::NaturalResource => Feature::RESOURCE,
            Self::Settlement => Feature::SETTLEMENT,
            Self::Mine => Feature::MINE,
            Self::Dungeon => Feature::STAIRS_DOWN,
        }
    }

    pub fn enterable_location_kind(self) -> Option<LocationKind> {
        matches!(self, Self::Dungeon).then_some(LocationKind::Dungeon)
    }
}

#[derive(Clone, Debug)]
pub struct PointOfInterest {
    pub pos: Pos,
    pub kind: PoiKind,
    pub location: Option<LocationId>,
    pub seed: u64,
    pub label: String,
    pub discovered: bool,
}

impl PointOfInterest {
    pub fn new(pos: Pos, kind: PoiKind, seed: u64) -> Self {
        Self {
            pos,
            kind,
            location: None,
            seed,
            label: kind.label().to_string(),
            discovered: false,
        }
    }

    pub fn with_location(mut self, location: LocationId) -> Self {
        self.location = Some(location);
        self
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    pub fn is_created(&self) -> bool {
        self.location.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_supported_dungeon_pois_are_enterable() {
        assert_eq!(
            PoiKind::Dungeon.enterable_location_kind(),
            Some(LocationKind::Dungeon)
        );
        assert_eq!(PoiKind::Ruins.enterable_location_kind(), None);
    }

    #[test]
    fn content_keys_map_to_local_kinds() {
        assert_eq!(
            PoiKind::from_content_key(&ContentKey::new("core:ruins").unwrap()),
            PoiKind::Ruins
        );
        assert_eq!(
            PoiKind::from_content_key(&ContentKey::new("core:dungeon").unwrap()),
            PoiKind::Dungeon
        );
    }
}
