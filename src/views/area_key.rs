use super::view_mode::ViewMode;
use hecs::Entity;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AreaKey {
  entity: Entity,
  level: Option<i32>,
}

impl AreaKey {
  pub fn new(entity: Entity, level: Option<i32>) -> Self {
    Self { entity, level }
  }
}

impl From<ViewMode> for Option<AreaKey> {
  fn from(mode: ViewMode) -> Self {
    match mode {
      ViewMode::World => None,
      ViewMode::Location { entity, level } => Some(AreaKey::new(entity, level)),
    }
  }
}
