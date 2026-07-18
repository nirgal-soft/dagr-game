use std::collections::HashMap;

use anyhow::{Context, Result};
use dagr_lib::components::world::{
  dungeon::Dungeon,
  hex::Hex,
  location::{Location, LocationType},
  spatial::Spatial,
  wilderness::Wilderness,
};
use dagr_lib::core::registry::EntityKind;
use dagr_lib::ems::{component::Component, entity_manager::EntityManager};
use hecs::Entity;
use serde_json::json;
use tracing::info;

use crate::areas::{Area, LocationConfig, PointOfInterest, Pos};
use crate::errors::ViewError;
use crate::generators::{dungeon::DungeonGenerator, wilderness::WildernessGenerator};
use crate::seed::{LocationDiscriminator, derive_seed};

use super::area_key::AreaKey;
use super::transition::{AsyncWork, Transition, TransitionIntent, TransitionOutcome};
use super::view_mode::ViewMode;

pub struct ViewManager {
  pub mode: ViewMode,
  area_cache: HashMap<AreaKey, Area>,
}

impl ViewManager {
  pub fn new() -> Self {
    Self {
      mode: ViewMode::World,
      area_cache: HashMap::new(),
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
        entity_manager.find_child_entity::<Wilderness>(location_data.get_id()?)
      {
        return self.enter(wilderness, None, entity_manager);
      }

      return Ok(TransitionOutcome::NeedsAsync(AsyncWork::CreateLocation {
        parent_entity: entity,
        location_type: LocationType::Wilderness,
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

    if let Some(wilderness_entity) = entity_manager.find_child_entity::<Wilderness>(parent_id) {
      return self.enter_wilderness_from_dungeon(wilderness_entity, dungeon_entity, entity_manager);
    }

    Ok(TransitionOutcome::NeedsAsync(AsyncWork::CreateLocation {
      parent_entity: hex_entity,
      location_type: LocationType::Wilderness,
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
    let AsyncWork::CreateLocation {
      parent_entity,
      location_type,
    } = work;
    let entity = self
      .ensure_child_location(parent_entity, location_type.clone(), entity_manager)
      .await?;

    if location_type == LocationType::Dungeon {
      if let Some(pos) = self.current_area().and_then(|area| area.stairs_down) {
        self.bind_dungeon_to_current_poi(pos, entity);
      }
    }

    if location_type == LocationType::Wilderness {
      if let Some(dungeon_entity) = origin.filter(|origin| entity_manager.has::<Dungeon>(*origin)) {
        return self.enter_wilderness_from_dungeon(entity, dungeon_entity, entity_manager);
      }
    }

    info!(?location_type, ?entity, "created child location");
    self.enter(entity, None, entity_manager)
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
      LocationType::Wilderness => entity_manager.find_child_entity::<Wilderness>(hex_id),
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

    let (kind, seed_data) = match location_type {
      LocationType::Wilderness => (
        EntityKind::Wilderness,
        json!({
          "world_seed": seed,
          "x": hex_spatial.get_x(),
          "y": hex_spatial.get_y(),
          "parent_location_id": hex_id,
        }),
      ),
      LocationType::Dungeon => (
        EntityKind::Dungeon,
        json!({
          "seed": seed,
          "depth_levels": 3,
          "x": hex_spatial.get_x(),
          "y": hex_spatial.get_y(),
          "parent_location_id": hex_id,
        }),
      ),
      _ => unreachable!(),
    };

    entity_manager.create_entity(kind, seed_data).await
  }
}

impl Default for ViewManager {
  fn default() -> Self {
    Self::new()
  }
}
