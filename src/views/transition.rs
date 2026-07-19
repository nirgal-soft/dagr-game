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
  CrossWildernessBoundary { target: Pos },
  EnterWildernessArea {
    parent_entity: Entity,
    area_x: i32,
    area_y: i32,
    crossing: WildernessCrossing,
  },
}
#[derive(Clone, Debug)]
pub enum TransitionOutcome {
  Ok(Transition),
  AtWorldLevel,
  NotAtExit,
  NoEntry,
  Unsupported,
  WorldBoundary {
    world_dx: i32,
    world_dy: i32,
    target_area_x: i32,
    target_area_y: i32,
    crossing: WildernessCrossing,
  },
  NeedsAsync(AsyncWork),
}

#[derive(Clone, Debug)]
pub enum AsyncWork {
  CreateLocation {
    parent_entity: Entity,
    location_type: LocationType,
  },
  CreateWildernessArea {
    parent_entity: Entity,
    area_x: i32,
    area_y: i32,
    width: i32,
    length: i32,
    crossing: WildernessCrossing,
  },
}

#[derive(Clone, Copy, Debug)]
pub struct WildernessCrossing {
  pub area_dx: i32,
  pub area_dy: i32,
  pub attempted_tile: Pos,
}

#[derive(Clone, Debug)]
pub struct Transition {
  pub player_pos: Pos,
  pub clear_fov: bool,
}
