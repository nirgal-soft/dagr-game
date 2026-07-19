use std::collections::HashMap;

use anyhow::{Context, Result};
use dagr_lib::components::world::{
  dungeon::Dungeon,
  hex::Hex,
  location::{Location, LocationType},
  spatial::Spatial,
  wilderness::Wilderness,
};
use dagr_lib::ems::{component::Component, entity_manager::EntityManager};
use dagr_lib::factories::world::{
  dungeon::DungeonSeed,
  wilderness::WildernessAreaSeed,
};
use hecs::Entity;
use tracing::info;

use crate::areas::{Area, LocationConfig, PointOfInterest, Pos};
use crate::errors::ViewError;
use crate::generators::{dungeon::DungeonGenerator, wilderness::WildernessGenerator};
use crate::seed::{LocationDiscriminator, derive_seed};
use crate::wilderness_layout::WildernessLayout;

use super::area_key::AreaKey;
use super::transition::{
  AsyncWork, Transition, TransitionIntent, TransitionOutcome, WildernessCrossing,
};
use super::view_mode::ViewMode;

pub struct ViewManager {
  pub mode: ViewMode,
  area_cache: HashMap<AreaKey, Area>,
  wilderness_layout: WildernessLayout,
}

impl ViewManager {
  pub fn new(wilderness_layout: WildernessLayout) -> Self {
    Self {
      mode: ViewMode::World,
      area_cache: HashMap::new(),
      wilderness_layout,
    }
  }

  pub fn current_area(&self) -> Option<&Area> {
    let key: Option<AreaKey> = self.mode.into();
    key.and_then(|key| self.area_cache.get(&key))
  }

  pub fn current_area_mut(&mut self) -> Option<&mut Area> {
    let key: Option<AreaKey> = self.mode.into();
    key.and_then(|key| self.area_cache.get_mut(&key))
  }

  pub fn is_in_world(&self) -> bool {
    matches!(self.mode, ViewMode::World)
  }

  pub fn is_in_location(&self) -> bool {
    matches!(self.mode, ViewMode::Location { .. })
  }

  pub fn current_entity(&self) -> Option<Entity> {
    match self.mode {
      ViewMode::World => None,
      ViewMode::Location { entity, .. } => Some(entity),
    }
  }

  pub fn current_level(&self) -> Option<i32> {
    match self.mode {
      ViewMode::Location { level, .. } => level,
      ViewMode::World => None,
    }
  }

  pub fn transition(
    &mut self,
    intent: TransitionIntent,
    player_pos: Pos,
    entity_manager: &EntityManager,
  ) -> Result<TransitionOutcome> {
    match intent {
      TransitionIntent::Enter(entity) => self.enter_entity(entity, entity_manager),
      TransitionIntent::Exit => self.exit(entity_manager),
      TransitionIntent::Ascend => self.ascend(player_pos, entity_manager),
      TransitionIntent::Descend => self.descend(player_pos, entity_manager),
      TransitionIntent::ToLevel(level) => self.transition_to_level(level, entity_manager),
      TransitionIntent::CrossWildernessBoundary { target } => {
        self.cross_wilderness_boundary(target, entity_manager)
      }
      TransitionIntent::EnterWildernessArea {
        parent_entity,
        area_x,
        area_y,
        crossing,
      } => self.enter_wilderness_area(
        parent_entity,
        area_x,
        area_y,
        crossing,
        entity_manager,
      ),
    }
  }

  fn enter_entity(
    &mut self,
    entity: Entity,
    entity_manager: &EntityManager,
  ) -> Result<TransitionOutcome> {
    let location = entity_manager
      .get_component::<Location, _>(entity)
      .context("failed to get location component")?;
    let location_data = location.get();

    if location_data.get_location_type() == LocationType::Hex {
      if let Some(wilderness) =
        entity_manager.find_wilderness_area(location_data.get_id()?, "origin")
      {
        return self.enter(wilderness, None, entity_manager);
      }

      return Ok(TransitionOutcome::NeedsAsync(AsyncWork::CreateWildernessArea {
        parent_entity: entity,
        area_x: 0,
        area_y: 0,
        width: self.wilderness_layout.area_width,
        length: self.wilderness_layout.area_height,
        crossing: WildernessCrossing{area_dx: 0, area_dy: 0, attempted_tile: (5, 5)},
      }));
    }

    self.enter(entity, None, entity_manager)
  }

  fn enter(
    &mut self,
    entity: Entity,
    level: Option<i32>,
    entity_manager: &EntityManager,
  ) -> Result<TransitionOutcome> {
    let location = entity_manager
      .get_component::<Location, _>(entity)
      .context("failed to get location component")?;
    let location_type = location.get().get_location_type();
    let config = LocationConfig::for_type(location_type.clone());
    let level = config.has_levels.then_some(level.unwrap_or(1));
    let key = AreaKey::new(entity, level);

    let spawn = match self.area_cache.get(&key) {
      Some(area) => Self::get_entry_spawn(area),
      None => {
        let area = self.generate_area(entity, location_type, level, entity_manager)?;
        let spawn = Self::get_entry_spawn(&area);
        self.area_cache.insert(key, area);
        spawn
      }
    };

    self.mode = ViewMode::Location { entity, level };

    Ok(TransitionOutcome::Ok(Transition {
      player_pos: spawn,
      clear_fov: config.has_fov,
    }))
  }

  fn exit(&mut self, entity_manager: &EntityManager) -> Result<TransitionOutcome> {
    let entity = match self.current_entity() {
      Some(entity) => entity,
      None => return Ok(TransitionOutcome::AtWorldLevel),
    };

    let location = entity_manager.get_component::<Location, _>(entity)?;
    match location.get().get_location_type() {
      LocationType::Wilderness => self.exit_wilderness(entity, entity_manager),
      LocationType::Dungeon => self.ascend_from_dungeon(entity_manager),
      _ => Ok(TransitionOutcome::Unsupported),
    }
  }

  fn ascend(
    &mut self,
    player_pos: Pos,
    entity_manager: &EntityManager,
  ) -> Result<TransitionOutcome> {
    let entity = match self.current_entity() {
      Some(entity) => entity,
      None => return Ok(TransitionOutcome::AtWorldLevel),
    };
    let location = entity_manager.get_component::<Location, _>(entity)?;

    match location.get().get_location_type() {
      LocationType::Wilderness => self.exit_wilderness(entity, entity_manager),
      LocationType::Dungeon => {
        if !self
          .current_area()
          .is_some_and(|area| area.is_stairs_up(player_pos.0, player_pos.1))
        {
          return Ok(TransitionOutcome::NotAtExit);
        }
        self.ascend_from_dungeon(entity_manager)
      }
      _ => Ok(TransitionOutcome::Unsupported),
    }
  }

  fn ascend_from_dungeon(&mut self, entity_manager: &EntityManager) -> Result<TransitionOutcome> {
    let dungeon_entity = self.current_entity().ok_or(ViewError::NoCurrentLocation)?;
    let current_level = self.current_level().unwrap_or(1);

    if current_level > 1 {
      let target_level = current_level - 1;
      let mut outcome = self.enter(dungeon_entity, Some(target_level), entity_manager)?;
      if let TransitionOutcome::Ok(ref mut transition) = outcome {
        if let Some(stairs_down) = self.current_area().and_then(|area| area.stairs_down) {
          transition.player_pos = stairs_down;
        }
      }
      return Ok(outcome);
    }

    let dungeon_location = entity_manager.get_component::<Location, _>(dungeon_entity)?;
    let parent_id = dungeon_location
      .get()
      .get_parent_location_id()?
      .ok_or(ViewError::NoParentLocation)?;
    let hex_entity = entity_manager
      .find_entity_by_location_id::<Hex>(parent_id)
      .ok_or(ViewError::LocationNotFound(parent_id))?;

    if let Some(wilderness_entity) = entity_manager.find_wilderness_area(parent_id, "origin") {
      return self.enter_wilderness_from_dungeon(wilderness_entity, dungeon_entity, entity_manager);
    }

    Ok(TransitionOutcome::NeedsAsync(AsyncWork::CreateWildernessArea {
      parent_entity: hex_entity,
      area_x: 0,
      area_y: 0,
      width: self.wilderness_layout.area_width,
      length: self.wilderness_layout.area_height,
      crossing: WildernessCrossing{area_dx: 0, area_dy: 0, attempted_tile: (5, 5)},
    }))
  }

  fn descend(
    &mut self,
    player_pos: Pos,
    entity_manager: &EntityManager,
  ) -> Result<TransitionOutcome> {
    let entity = match self.current_entity() {
      Some(entity) => entity,
      None => return Ok(TransitionOutcome::Unsupported),
    };
    let location = entity_manager.get_component::<Location, _>(entity)?;

    match location.get().get_location_type() {
      LocationType::Wilderness => self.descend_from_wilderness(entity, player_pos, entity_manager),
      LocationType::Dungeon => self.descend_in_dungeon(entity, player_pos, entity_manager),
      _ => Ok(TransitionOutcome::Unsupported),
    }
  }

  fn descend_from_wilderness(
    &mut self,
    wilderness_entity: Entity,
    player_pos: Pos,
    entity_manager: &EntityManager,
  ) -> Result<TransitionOutcome> {
    let area = self.current_area().ok_or(ViewError::AreaNotCached)?;
    if !area.is_stairs_down(player_pos.0, player_pos.1) {
      return Ok(TransitionOutcome::NotAtExit);
    }

    if let Some(dungeon_entity) = area.get_poi_at(player_pos).and_then(|poi| poi.entity) {
      return self.enter(dungeon_entity, Some(1), entity_manager);
    }

    let wilderness_location = entity_manager.get_component::<Location, _>(wilderness_entity)?;
    let parent_id = wilderness_location
      .get()
      .get_parent_location_id()?
      .ok_or(ViewError::NoParentLocation)?;
    let hex_entity = entity_manager
      .find_entity_by_location_id::<Hex>(parent_id)
      .ok_or(ViewError::LocationNotFound(parent_id))?;

    if let Some(dungeon_entity) = entity_manager.find_child_entity::<Dungeon>(parent_id) {
      self.bind_dungeon_to_current_poi(player_pos, dungeon_entity);
      return self.enter(dungeon_entity, Some(1), entity_manager);
    }

    Ok(TransitionOutcome::NeedsAsync(AsyncWork::CreateLocation {
      parent_entity: hex_entity,
      location_type: LocationType::Dungeon,
    }))
  }

  fn descend_in_dungeon(
    &mut self,
    dungeon_entity: Entity,
    player_pos: Pos,
    entity_manager: &EntityManager,
  ) -> Result<TransitionOutcome> {
    let area = self.current_area().ok_or(ViewError::AreaNotCached)?;
    if !area.is_stairs_down(player_pos.0, player_pos.1) {
      return Ok(TransitionOutcome::NotAtExit);
    }

    let current_level = self.current_level().unwrap_or(1);
    if current_level >= area.max_level.unwrap_or(current_level) {
      return Ok(TransitionOutcome::NoEntry);
    }

    let target_level = current_level + 1;
    let mut outcome = self.enter(dungeon_entity, Some(target_level), entity_manager)?;
    if let TransitionOutcome::Ok(ref mut transition) = outcome {
      if let Some(stairs_up) = self.current_area().and_then(|area| area.stairs_up) {
        transition.player_pos = stairs_up;
      }
    }
    Ok(outcome)
  }

  fn transition_to_level(
    &mut self,
    level: i32,
    entity_manager: &EntityManager,
  ) -> Result<TransitionOutcome> {
    let entity = self.current_entity().ok_or(ViewError::NoCurrentLocation)?;
    let location = entity_manager.get_component::<Location, _>(entity)?;
    if location.get().get_location_type() != LocationType::Dungeon || level < 1 {
      return Ok(TransitionOutcome::Unsupported);
    }
    self.enter(entity, Some(level), entity_manager)
  }

  fn cross_wilderness_boundary(
    &mut self,
    attempted_tile: Pos,
    entity_manager: &EntityManager,
  ) -> Result<TransitionOutcome> {
    let wilderness_entity = self.current_entity().ok_or(ViewError::NoCurrentLocation)?;
    if !entity_manager.has::<Wilderness>(wilderness_entity) {
      return Ok(TransitionOutcome::Unsupported);
    }
    let current_spatial = entity_manager.get_component::<Spatial, _>(wilderness_entity)?.get();
    let area_dx = if attempted_tile.0 < 0 {
      -1
    } else if attempted_tile.0 >= current_spatial.get_width() {
      1
    } else {
      0
    };
    let area_dy = if attempted_tile.1 < 0 {
      -1
    } else if attempted_tile.1 >= current_spatial.get_length() {
      1
    } else {
      0
    };
    if area_dx == 0 && area_dy == 0 {
      return Ok(TransitionOutcome::Unsupported);
    }
    let location = entity_manager.get_component::<Location, _>(wilderness_entity)?.get();
    let parent_id = location
      .get_parent_location_id()?
      .ok_or(ViewError::NoParentLocation)?;
    let parent_entity = entity_manager
      .find_entity_by_location_id::<Hex>(parent_id)
      .ok_or(ViewError::LocationNotFound(parent_id))?;
    let target_x = current_spatial.get_x() + area_dx;
    let target_y = current_spatial.get_y() + area_dy;
    let crossing = WildernessCrossing { area_dx, area_dy, attempted_tile };
    if !self.wilderness_layout.contains(target_x, target_y) {
      let (world_dx, target_area_x) = wrap_area_axis(
        target_x,
        self.wilderness_layout.min_x(),
        self.wilderness_layout.max_x(),
      );
      let (world_dy, target_area_y) = wrap_area_axis(
        target_y,
        self.wilderness_layout.min_y(),
        self.wilderness_layout.max_y(),
      );
      return Ok(TransitionOutcome::WorldBoundary {
        world_dx,
        world_dy,
        target_area_x,
        target_area_y,
        crossing,
      });
    }
    let area_key = wilderness_area_key(target_x, target_y);

    if let Some(target) = entity_manager.find_wilderness_area(parent_id, &area_key) {
      return self.enter_wilderness_from_boundary(target, crossing, entity_manager);
    }

    Ok(TransitionOutcome::NeedsAsync(AsyncWork::CreateWildernessArea {
      parent_entity,
      area_x: target_x,
      area_y: target_y,
      width: current_spatial.get_width(),
      length: current_spatial.get_length(),
      crossing,
    }))
  }

  fn enter_wilderness_area(
    &mut self,
    parent_entity: Entity,
    area_x: i32,
    area_y: i32,
    crossing: WildernessCrossing,
    entity_manager: &EntityManager,
  ) -> Result<TransitionOutcome> {
    let parent_id = entity_manager
      .get_component::<Location, _>(parent_entity)?
      .get()
      .get_id()?;
    let area_key = wilderness_area_key(area_x, area_y);
    if let Some(entity) = entity_manager.find_wilderness_area(parent_id, &area_key) {
      return self.enter_wilderness_from_boundary(entity, crossing, entity_manager);
    }
    Ok(TransitionOutcome::NeedsAsync(AsyncWork::CreateWildernessArea {
      parent_entity,
      area_x,
      area_y,
      width: self.wilderness_layout.area_width,
      length: self.wilderness_layout.area_height,
      crossing,
    }))
  }

  fn enter_wilderness_from_boundary(
    &mut self,
    wilderness_entity: Entity,
    crossing: WildernessCrossing,
    entity_manager: &EntityManager,
  ) -> Result<TransitionOutcome> {
    let target_spatial = entity_manager.get_component::<Spatial, _>(wilderness_entity)?.get();
    let arrival = wilderness_arrival(
      crossing,
      target_spatial.get_width(),
      target_spatial.get_length(),
    );
    let mut outcome = self.enter(wilderness_entity, None, entity_manager)?;
    if let TransitionOutcome::Ok(ref mut transition) = outcome {
      transition.player_pos = arrival;
      if let Some(area) = self.current_area_mut() {
        area.remove_feature(arrival.0, arrival.1);
      }
    }
    Ok(outcome)
  }

  fn exit_wilderness(
    &mut self,
    wilderness_entity: Entity,
    entity_manager: &EntityManager,
  ) -> Result<TransitionOutcome> {
    let location = entity_manager.get_component::<Location, _>(wilderness_entity)?;
    let parent_id = location
      .get()
      .get_parent_location_id()?
      .ok_or(ViewError::NoParentLocation)?;
    let hex_entity = entity_manager
      .find_entity_by_location_id::<Hex>(parent_id)
      .ok_or(ViewError::LocationNotFound(parent_id))?;
    let spatial = entity_manager
      .get_component::<Spatial, _>(hex_entity)?
      .get();

    self.mode = ViewMode::World;
    Ok(TransitionOutcome::Ok(Transition {
      player_pos: (spatial.get_x(), spatial.get_y()),
      clear_fov: false,
    }))
  }

  fn enter_wilderness_from_dungeon(
    &mut self,
    wilderness_entity: Entity,
    dungeon_entity: Entity,
    entity_manager: &EntityManager,
  ) -> Result<TransitionOutcome> {
    let mut outcome = self.enter(wilderness_entity, None, entity_manager)?;
    if let TransitionOutcome::Ok(ref mut transition) = outcome {
      if let Some(pos) = self
        .current_area()
        .and_then(|area| area.find_poi_by_entity(dungeon_entity))
      {
        transition.player_pos = pos;
      }
    }
    Ok(outcome)
  }

  fn generate_area(
    &self,
    entity: Entity,
    location_type: LocationType,
    level: Option<i32>,
    entity_manager: &EntityManager,
  ) -> Result<Area> {
    match location_type {
      LocationType::Wilderness => {
        let location = entity_manager.get_component::<Location, _>(entity)?.get();
        let spatial = entity_manager.get_component::<Spatial, _>(entity)?.get();
        let seed = location.get_seed().unwrap_or(0) as u64;
        let mut area =
          WildernessGenerator::new(seed).generate(spatial.get_width(), spatial.get_length())?;
        let entrance = area.entrance.unwrap_or((area.width / 2, area.height / 2));
        area.set_stairs_down(entrance.0, entrance.1);

        let poi_seed = derive_seed(
          seed,
          LocationDiscriminator::new(LocationType::Dungeon, entrance.0, entrance.1),
        );
        let mut poi = PointOfInterest::new(entrance, LocationType::Dungeon, poi_seed)
          .with_label("Dungeon entrance");

        if let Some(parent_id) = location.get_parent_location_id()? {
          if let Some(dungeon) = entity_manager.find_child_entity::<Dungeon>(parent_id) {
            poi = poi.with_entity(dungeon);
          }
        }
        area.add_poi(poi);
        Ok(area)
      }
      LocationType::Dungeon => {
        let location = entity_manager.get_component::<Location, _>(entity)?.get();
        DungeonGenerator::new(location.get_seed().unwrap_or(0) as u64)
          .generate(entity, entity_manager, level.unwrap_or(1))
          .map_err(|error| ViewError::GenerationFailed(error.to_string()).into())
      }
      other => Err(ViewError::UnsupportedLocationType(other).into()),
    }
  }

  fn get_entry_spawn(area: &Area) -> Pos {
    area
      .entrance
      .or(area.stairs_up)
      .unwrap_or((area.width / 2, area.height / 2))
  }

  fn bind_dungeon_to_current_poi(&mut self, pos: Pos, dungeon_entity: Entity) {
    if let Some(poi) = self
      .current_area_mut()
      .and_then(|area| area.get_poi_at_mut(pos))
    {
      poi.entity = Some(dungeon_entity);
    }
  }

  pub async fn execute_async(
    &mut self,
    work: AsyncWork,
    entity_manager: &EntityManager,
  ) -> Result<TransitionOutcome> {
    let origin = self.current_entity();
    match work {
      AsyncWork::CreateLocation{parent_entity, location_type} => {
        let entity = self
          .ensure_child_location(parent_entity, location_type.clone(), entity_manager)
          .await?;
        if location_type == LocationType::Dungeon {
          if let Some(pos) = self.current_area().and_then(|area| area.stairs_down) {
            self.bind_dungeon_to_current_poi(pos, entity);
          }
        }
        info!(?location_type, ?entity, "created child location");
        self.enter(entity, None, entity_manager)
      }
      AsyncWork::CreateWildernessArea{
        parent_entity, area_x, area_y, width, length, crossing,
      } => {
        let entity = self.ensure_wilderness_area(
          parent_entity, area_x, area_y, width, length, entity_manager,
        ).await?;
        if let Some(dungeon_entity) = origin.filter(|origin| entity_manager.has::<Dungeon>(*origin)) {
          return self.enter_wilderness_from_dungeon(entity, dungeon_entity, entity_manager);
        }
        info!(?entity, area_x, area_y, "created wilderness area");
        if crossing.area_dx == 0 && crossing.area_dy == 0 {
          self.enter(entity, None, entity_manager)
        } else {
          self.enter_wilderness_from_boundary(entity, crossing, entity_manager)
        }
      }
    }
  }

  async fn ensure_child_location(
    &self,
    hex_entity: Entity,
    location_type: LocationType,
    entity_manager: &EntityManager,
  ) -> Result<Entity> {
    let hex_location = entity_manager
      .get_component::<Location, _>(hex_entity)
      .context("failed to get hex location")?
      .get();
    let hex_spatial = entity_manager
      .get_component::<Spatial, _>(hex_entity)
      .context("failed to get hex spatial")?
      .get();
    let hex_id = hex_location.get_id()?;

    let existing = match location_type {
      LocationType::Dungeon => entity_manager.find_child_entity::<Dungeon>(hex_id),
      _ => return Err(ViewError::UnsupportedLocationType(location_type).into()),
    };
    if let Some(entity) = existing {
      return Ok(entity);
    }

    let seed = derive_seed(
      hex_location.get_seed().unwrap_or(0) as u64,
      LocationDiscriminator::new(
        location_type.clone(),
        hex_spatial.get_x(),
        hex_spatial.get_y(),
      ),
    );

    match location_type {
      LocationType::Dungeon => entity_manager.create(DungeonSeed{
        seed,
        depth_levels: 3,
        x: hex_spatial.get_x(),
        y: hex_spatial.get_y(),
        parent_location_id: Some(hex_id),
      }).await,
      _ => unreachable!(),
    }
  }

  async fn ensure_wilderness_area(
    &self,
    parent_entity: Entity,
    area_x: i32,
    area_y: i32,
    width: i32,
    length: i32,
    entity_manager: &EntityManager,
  ) -> Result<Entity> {
    let parent_id = entity_manager
      .get_component::<Location, _>(parent_entity)
      .context("failed to get wilderness parent location")?
      .get()
      .get_id()?;
    let area_key = wilderness_area_key(area_x, area_y);
    if let Some(entity) = entity_manager.find_wilderness_area(parent_id, &area_key) {
      return Ok(entity);
    }
    entity_manager.create(WildernessAreaSeed{
      area_key,
      x: area_x,
      y: area_y,
      width,
      length,
      parent_location_id: Some(parent_id),
    }).await
  }
}

fn wrap_area_axis(value: i32, minimum: i32, maximum: i32) -> (i32, i32) {
  if value < minimum {
    (-1, maximum)
  } else if value > maximum {
    (1, minimum)
  } else {
    (0, value)
  }
}

fn wilderness_area_key(x: i32, y: i32) -> String {
  if x == 0 && y == 0 {
    "origin".to_string()
  } else {
    format!("grid:{x}:{y}")
  }
}

fn wilderness_arrival(crossing: WildernessCrossing, width: i32, length: i32) -> Pos {
  let x = match crossing.area_dx {
    value if value < 0 => width - 1,
    value if value > 0 => 0,
    _ => crossing.attempted_tile.0.clamp(0, width - 1),
  };
  let y = match crossing.area_dy {
    value if value < 0 => length - 1,
    value if value > 0 => 0,
    _ => crossing.attempted_tile.1.clamp(0, length - 1),
  };
  (x, y)
}

impl Default for ViewManager {
  fn default() -> Self {
    Self::new(WildernessLayout::default())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::sync::{Arc, Mutex};
  use dagr_lib::{
    components::world::{
      hex::{Hex, HexData},
      location::LocationData,
      spatial::SpatialData,
      wilderness::WildernessData,
    },
    core::registry::FactoryRegistry,
  };
  use hecs::World;
  use serde_json::json;

  fn location(id: i32, location_type: &str, parent: Option<i32>, seed: i64) -> Location {
    Location::new(serde_json::from_value::<LocationData>(json!({
      "id":id,"name":"test","description":"test","location_type":location_type,
      "location_status":"Unexplored","parent_location_id":parent,"is_persistent":true,
      "is_discoverable":true,"discovery_text":null,"seed":seed
    })).unwrap())
  }
  fn spatial(location_id: i32, x: i32, y: i32, width: i32, length: i32, container: Option<i32>) -> Spatial {
    Spatial::new(serde_json::from_value::<SpatialData>(json!({
      "id":location_id,"location_id":location_id,"x":x,"y":y,"z":null,
      "width":width,"length":length,"height":null,"container_id":container,
      "traversable":true,"movement_cost":1
    })).unwrap())
  }
  fn wilderness(id: i32, location_id: i32, parent: i32, key: &str) -> Wilderness {
    Wilderness::new(serde_json::from_value::<WildernessData>(json!({
      "id":id,"location_id":location_id,"parent_location_id":parent,
      "area_key":key,"feature":"test"
    })).unwrap())
  }

  #[test]
  fn wilderness_area_keys_are_stable_and_origin_is_named() {
    assert_eq!(wilderness_area_key(0, 0), "origin");
    assert_eq!(wilderness_area_key(-2, 3), "grid:-2:3");
  }

  #[test]
  fn existing_wilderness_siblings_can_be_traversed_in_both_directions() {
    let mut world = World::new();
    let hex_data: HexData = serde_json::from_value(json!({
      "id":1,"location_id":1,"terrain":1,"vegetation":1,"water":1,"poi":1,
      "climate":1,"region_id":null,"danger_level":1,"resource_richness":1
    })).unwrap();
    let hex = world.spawn((Hex::new(hex_data), location(1, "Hex", None, 100), spatial(1, 7, 8, 1, 1, None)));
    let origin = world.spawn((wilderness(1, 2, 1, "origin"), location(2, "Wilderness", Some(1), 101), spatial(2, 0, 0, 10, 10, Some(1))));
    let east = world.spawn((wilderness(2, 3, 1, "grid:1:0"), location(3, "Wilderness", Some(1), 102), spatial(3, 1, 0, 10, 10, Some(1))));
    let manager = EntityManager::from_world(Arc::new(Mutex::new(world)), Arc::new(FactoryRegistry::new()));
    let mut views = ViewManager::new(WildernessLayout::new(3, 3, 10, 10).unwrap());
    assert!(matches!(views.enter_entity(hex, &manager).unwrap(), TransitionOutcome::Ok(_)));
    assert_eq!(views.current_entity(), Some(origin));

    let eastward = views.transition(
      TransitionIntent::CrossWildernessBoundary{target:(10, 5)}, (9, 5), &manager,
    ).unwrap();
    let TransitionOutcome::Ok(eastward) = eastward else{panic!("expected east transition")};
    assert_eq!(eastward.player_pos, (0, 5));
    assert_eq!(views.current_entity(), Some(east));

    assert!(matches!(
      views.transition(
        TransitionIntent::CrossWildernessBoundary{target:(10, 5)}, (9, 5), &manager,
      ).unwrap(),
      TransitionOutcome::WorldBoundary{
        world_dx:1, world_dy:0, target_area_x:-1, target_area_y:0, ..
      }
    ));
    assert_eq!(views.current_entity(), Some(east));

    let westward = views.transition(
      TransitionIntent::CrossWildernessBoundary{target:(-1, 5)}, (0, 5), &manager,
    ).unwrap();
    let TransitionOutcome::Ok(westward) = westward else{panic!("expected west transition")};
    assert_eq!(westward.player_pos, (9, 5));
    assert_eq!(views.current_entity(), Some(origin));
  }

  #[test]
  fn single_area_layout_wraps_into_neighboring_world_container() {
    let layout = WildernessLayout::default();
    assert_eq!(wrap_area_axis(1, layout.min_x(), layout.max_x()), (1, 0));
    assert_eq!(wrap_area_axis(-1, layout.min_y(), layout.max_y()), (-1, 0));
  }

  #[test]
  fn wilderness_crossings_arrive_on_opposite_edges() {
    assert_eq!(
      wilderness_arrival(
        WildernessCrossing{area_dx: 1, area_dy: 0, attempted_tile: (10, 4)},
        10,
        10,
      ),
      (0, 4),
    );
    assert_eq!(
      wilderness_arrival(
        WildernessCrossing{area_dx: -1, area_dy: -1, attempted_tile: (-1, -1)},
        12,
        8,
      ),
      (11, 7),
    );
  }
}
