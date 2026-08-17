use super::Ground;
use dagr_lib::world::LocationKind;

pub struct LocationConfig {
  pub ground: Ground,
  pub has_fov: bool,
  pub has_levels: bool,
}

impl LocationConfig {
  pub fn for_type(location_type: LocationKind) -> Self {
    match location_type {
      LocationKind::Dungeon => Self {
        ground: Ground::DUNGEON,
        has_fov: true,
        has_levels: true,
      },
      LocationKind::Wilderness => Self {
        ground: Ground::WILDERNESS,
        has_fov: true,
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
