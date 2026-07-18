use super::Ground;
use dagr_lib::components::world::location::LocationType;

pub struct LocationConfig {
  pub ground: Ground,
  pub has_fov: bool,
  pub has_levels: bool,
}

impl LocationConfig {
  pub fn for_type(location_type: LocationType) -> Self {
    match location_type {
      LocationType::Dungeon => Self {
        ground: Ground::DUNGEON,
        has_fov: true,
        has_levels: true,
      },
      LocationType::Wilderness => Self {
        ground: Ground::WILDERNESS,
        has_fov: false,
        has_levels: false,
      },
      _ => Self {
        ground: Ground::WILDERNESS,
        has_fov: false,
        has_levels: false,
      },
    }
  }
}
