use crate::areas::Pos;
use dagr_lib::components::world::location::LocationType;
use hecs::Entity;

#[derive(Clone, Copy, Debug)]
pub enum TransitionIntent {
  Enter(Entity),
  Exit,
  Ascend,
  Descend,
  ToLevel(i32),
}
#[derive(Clone, Debug)]
pub enum TransitionOutcome {
  Ok(Transition),
  AtWorldLevel,
  NotAtExit,
  NoEntry,
  Unsupported,
  NeedsAsync(AsyncWork),
}

#[derive(Clone, Debug)]
pub enum AsyncWork {
  CreateLocation {
    parent_entity: Entity,
    location_type: LocationType,
  },
}

#[derive(Clone, Debug)]
pub struct Transition {
  pub player_pos: Pos,
  pub clear_fov: bool,
}
