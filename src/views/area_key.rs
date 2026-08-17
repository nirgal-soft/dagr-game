use dagr_lib::world::LocationId;

use super::view_mode::ViewMode;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AreaKey {
  location: LocationId,
  level: Option<i32>,
}

impl AreaKey {
  pub fn new(location: LocationId, level: Option<i32>) -> Self {
    Self { location, level }
  }
}

impl From<ViewMode> for Option<AreaKey> {
  fn from(mode: ViewMode) -> Self {
    match mode {
      ViewMode::World => None,
      ViewMode::Location { location, level } => Some(AreaKey::new(location, level)),
    }
  }
}
