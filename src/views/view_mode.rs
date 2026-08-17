use dagr_lib::world::LocationId;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ViewMode {
  World,
  Location {
    location: LocationId,
    level: Option<i32>,
  },
}

impl ViewMode {
  pub fn location(location: LocationId) -> Self {
    Self::Location {
      location,
      level: None,
    }
  }

  pub fn location_with_level(location: LocationId, level: i32) -> Self {
    Self::Location {
      location,
      level: Some(level),
    }
  }
}
