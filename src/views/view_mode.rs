use hecs::Entity;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ViewMode {
  World,
  Location { entity: Entity, level: Option<i32> },
}

impl ViewMode {
  pub fn location(entity: Entity) -> Self {
    Self::Location {
      entity,
      level: None,
    }
  }

  pub fn location_with_level(entity: Entity, level: i32) -> Self {
    Self::Location {
      entity,
      level: Some(level),
    }
  }
}
