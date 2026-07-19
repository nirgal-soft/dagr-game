use super::{Feature, Pos};
use dagr_lib::{
  components::world::location::LocationType,
  kits::hexkit::poi::PointOfInterest as HexPointOfInterest,
};
use hecs::Entity;

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
  pub fn from_hex_poi(poi: &HexPointOfInterest) -> Self {
    match poi {
      HexPointOfInterest::Ruins => Self::Ruins,
      HexPointOfInterest::Caves => Self::Cave,
      HexPointOfInterest::Tomb => Self::Tomb,
      HexPointOfInterest::Lair => Self::Lair,
      HexPointOfInterest::NatResource(_) => Self::NaturalResource,
      HexPointOfInterest::Settlement => Self::Settlement,
      HexPointOfInterest::Mine => Self::Mine,
      HexPointOfInterest::Dungeon => Self::Dungeon,
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

  pub fn enterable_location_type(self) -> Option<LocationType> {
    matches!(self, Self::Dungeon).then_some(LocationType::Dungeon)
  }
}

#[derive(Clone, Debug)]
pub struct PointOfInterest {
  pub pos: Pos,
  pub kind: PoiKind,
  pub entity: Option<Entity>,
  pub seed: u64,
  pub label: String,
  pub discovered: bool,
}

impl PointOfInterest {
  pub fn new(pos: Pos, kind: PoiKind, seed: u64) -> Self {
    Self {
      pos,
      kind,
      entity: None,
      seed,
      label: kind.label().to_string(),
      discovered: false,
    }
  }

  pub fn with_entity(mut self, entity: Entity) -> Self {
    self.entity = Some(entity);
    self
  }

  pub fn with_label(mut self, label: impl Into<String>) -> Self {
    self.label = label.into();
    self
  }

  pub fn is_created(&self) -> bool {
    self.entity.is_some()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn only_supported_dungeon_pois_are_enterable() {
    assert_eq!(PoiKind::Dungeon.enterable_location_type(), Some(LocationType::Dungeon));
    assert_eq!(PoiKind::Ruins.enterable_location_type(), None);
  }

  #[test]
  fn hex_pois_map_to_local_kinds() {
    assert_eq!(PoiKind::from_hex_poi(&HexPointOfInterest::Ruins), PoiKind::Ruins);
    assert_eq!(PoiKind::from_hex_poi(&HexPointOfInterest::Dungeon), PoiKind::Dungeon);
  }
}
