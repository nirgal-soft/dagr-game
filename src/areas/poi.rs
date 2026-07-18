use super::Pos;
use dagr_lib::components::world::location::LocationType;
use hecs::Entity;

#[derive(Clone, Debug)]
pub struct PointOfInterest {
  pub pos: Pos,
  pub location_type: LocationType,
  pub entity: Option<Entity>,
  pub seed: u64,
  pub label: Option<String>,
}

impl PointOfInterest {
  pub fn new(pos: Pos, location_type: LocationType, seed: u64) -> Self {
    Self {
      pos,
      location_type,
      entity: None,
      seed,
      label: None,
    }
  }

  pub fn with_entity(mut self, entity: Entity) -> Self {
    self.entity = Some(entity);
    self
  }

  pub fn with_label(mut self, label: impl Into<String>) -> Self {
    self.label = Some(label.into());
    self
  }

  pub fn is_created(&self) -> bool {
    self.entity.is_some()
  }
}
