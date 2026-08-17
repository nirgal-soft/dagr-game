use dagr_lib::world::LocationKind;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub fn derive_seed(parent_seed: u64, discriminator: impl Hash) -> u64 {
  let mut hasher = DefaultHasher::new();
  parent_seed.hash(&mut hasher);
  discriminator.hash(&mut hasher);
  hasher.finish()
}

#[derive(Clone, Debug, Hash)]
pub struct LocationDiscriminator {
  pub location_type: u8,
  pub x: i32,
  pub y: i32,
  pub index: u32,
}

impl LocationDiscriminator {
  pub fn new(location_kind: LocationKind, x: i32, y: i32) -> Self {
    Self {
      location_type: location_discriminator(location_kind),
      x,
      y,
      index: 0,
    }
  }
}

fn location_discriminator(kind: LocationKind) -> u8 {
  match kind {
    LocationKind::Hex => 0,
    LocationKind::Dungeon => 1,
    LocationKind::DungeonRoom => 2,
    LocationKind::Town => 3,
    LocationKind::Building => 4,
    LocationKind::Interior => 5,
    LocationKind::Wilderness => 6,
    LocationKind::Cave => 7,
    LocationKind::Lair => 8,
  }
}

#[derive(Clone, Debug, Hash)]
pub struct LevelDiscriminator {
  pub level: i32,
}

impl LevelDiscriminator {
  pub fn new(level: i32) -> Self {
    Self { level }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn derived_seeds_are_repeatable_and_distinct() {
    let location = LocationDiscriminator::new(LocationKind::Dungeon, 4, -2);
    assert_eq!(derive_seed(42, &location), derive_seed(42, &location));
    assert_ne!(
      derive_seed(42, &location),
      derive_seed(42, LevelDiscriminator::new(2)),
    );
    assert_ne!(derive_seed(42, &location), derive_seed(43, &location));
  }
}
