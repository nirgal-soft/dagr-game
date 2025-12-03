use std::collections::HashMap;
use std::collections::hash_map::Entry;
use anyhow::{anyhow, Result};
use hecs::Entity;
use serde_json::json;
use tracing::{debug, info};
use dagr_lib::components::world::{
  dungeon::Dungeon,
  hex::Hex,
  location::Location,
  spatial::Spatial,
  wilderness::Wilderness
};
use dagr_lib::core::registry::EntityKind;
use dagr_lib::ems::{entity_manager::EntityManager, component::Component};
use crate::camera::Camera;
use crate::dungeon_generator::{DungeonArea, DungeonGenerator};
use crate::renderer::{Tile, RenderConfig};
use crate::visiblity::VisibilityMap;
use crate::wilderness_generator::{WildernessArea, WildernessGenerator};
use crate::world_map::WorldMap;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ViewMode{
  HexMap,
  Wilderness(Entity),
  Dungeon(Entity),
}

pub struct GameState{
  pub entity_manager: EntityManager,
  pub map: WorldMap,
  pub camera: Camera,
  pub player_x: i32,
  pub player_y: i32,
  pub view_mode: ViewMode,
  pub render_config: RenderConfig,
  pub visibility: VisibilityMap,
  wilderness_cache: HashMap<Entity, WildernessArea>,
  dungeon_cache: HashMap<(Entity, i32), DungeonArea>,
}

impl GameState{
  pub fn new(entity_manager: EntityManager, view_w: u16, view_h: u16) -> Self{
    let mut state = Self{
      entity_manager,
      map: WorldMap::new(),
      camera: Camera::new(view_w, view_h),
      player_x: 0,
      player_y: 0,
      view_mode: ViewMode::HexMap,
      render_config: RenderConfig::default(),
      visibility: VisibilityMap::new(8),
      wilderness_cache: HashMap::new(),
      dungeon_cache: HashMap::new(),
    };
    state.rebuild_map();
    state.attach_tiles();
    state
  }

  pub fn update_visibility(&mut self){
    if let ViewMode::Dungeon(dungeon_entity) = self.view_mode{
      let current_level = self.dungeon_cache
        .iter()
        .find(|((e, _), _)| *e == dungeon_entity)
        .map(|((_, l), _)| *l)
        .unwrap_or(1);

      if let Some(area) = self.dungeon_cache.get_mut(&(dungeon_entity, current_level)){
        self.visibility.update(self.player_x, self.player_y, |x, y|{
          area.is_opaque(x, y)
        });

        let visible: Vec<_> = self.visibility.visible_tiles().collect();
        area.mark_visible_as_seen(visible);
      }
    }
  }

  pub fn get_visible_dungeon_tile(&self, x: i32, y: i32) -> Option<Tile>{
    if let ViewMode::Dungeon(dungeon_entity) = self.view_mode{
      let current_level = self.dungeon_cache
        .iter()
        .find(|((e, _), _)| *e == dungeon_entity)
        .map(|((_, l), _)| *l)
        .unwrap_or(1);

      if let Some(dungeon) = self.dungeon_cache.get(&(dungeon_entity, current_level)){
        let currently_visible = self.visibility.is_visible(x, y);
        return dungeon.get_visible_tile(x, y, currently_visible, &self.render_config);
      }
    }
    None
  }

  pub async fn move_player(&mut self, dx: i32, dy: i32) -> Result<()>{
    info!("moving player by {}, {}", dx, dy);
    let new_x = self.player_x + dx;
    let new_y = self.player_y + dy;
    let mut can_move = true;

    match self.view_mode{
      ViewMode::HexMap => {
        info!("moving player in hexmap");
        if self.map.get((new_x, new_y)).is_none(){
          self.generate_hex_at(new_x, new_y).await?;
        }
      }
      ViewMode::Wilderness(wilderness_entity) => {
        if let Some(wilderness) = self.wilderness_cache.get(&wilderness_entity){
          if !wilderness.contains(new_x, new_y){
            can_move = false;
          }
        }
      }
      ViewMode::Dungeon(dungeon_entity) => {
        if let Some(dungeon) = self.dungeon_cache.get(&(dungeon_entity, 1)){
          if !dungeon.is_walkable(new_x, new_y){
            can_move = false;
          }
        }
      }
    }

    if can_move{
      self.player_x = new_x;
      self.player_y = new_y;
      self.camera.center_on(new_x, new_y);
    }

    Ok(())
  }

  pub async fn enter_wilderness(&mut self) -> Result<()>{
    let hex_entity = self.map.get((self.player_x, self.player_y))
      .ok_or(anyhow!("No hex found at player position"))?;

    let hex_location = self.entity_manager.get_component::<Location, _>(hex_entity)?;
    let hex_spatial = self.entity_manager.get_component::<Spatial, _>(hex_entity)?;
    let hex_location_id = hex_location.get().get_id();
    let hex_seed = hex_location.get().get_seed().unwrap_or(0);
    info!("hex location id: {}", hex_location_id);

    let wilderness_entity = match self.entity_manager.find_child_entity::<Wilderness>(hex_location_id){
      Some(entity) => {
        info!("found wilderness entity");
        entity
      }
      None => {
        info!("no wilderness entity found");
        self.entity_manager.create_entity(
          EntityKind::Wilderness,
          json!({
            "x": hex_spatial.get().get_x(),
            "y": hex_spatial.get().get_y(),
            "parent_location_id": Some(hex_location_id)
          })
        ).await?
      }
    };

    let _dungeon_entity = match self.entity_manager.find_child_entity::<Dungeon>(hex_location_id){
      Some(entity) => {
        info!("found dungeon entity");
        entity
      }
      None => {
        info!("no dungeon entity found");
        self.entity_manager.create_entity(
          EntityKind::Dungeon,
          json!({
            "seed": hex_seed,
            "depth_levels": 3,
            "x": hex_spatial.get().get_x(),
            "y": hex_spatial.get().get_y(),
            "parent_location_id": Some(hex_location_id)
          })
        ).await?
      }
    };

    let spatial = self.entity_manager.get_component::<Spatial, _>(wilderness_entity)?;
    let wilderness = self.entity_manager.get_component::<Wilderness, _>(wilderness_entity)?;
    info!("wilderness entity: {:?}", wilderness_entity);
    info!("wilderness component: {:?}", wilderness);

    if let Entry::Vacant(e) = self.wilderness_cache.entry(wilderness_entity) {
      info!("wilderness not cached, generating");
      let seed = hex_location.get().get_seed().unwrap_or(0);
      let spatial_data = spatial.get();
      let generator = WildernessGenerator::new(seed as u64);
      let mut area = generator.generate(spatial_data.get_width(), spatial_data.get_length())?;

      let entrance_x = area.width / 2;
      let entrance_y = area.height / 2;
      area.set_dungeon_entrance(entrance_x, entrance_y);
      info!("dungeon entrance at {},{}", entrance_x, entrance_y);

      e.insert(area);
    }

    self.view_mode = ViewMode::Wilderness(wilderness_entity);

    let wilderness = self.wilderness_cache.get(&wilderness_entity)
      .ok_or_else(|| anyhow!("no wilderness found at hex location"))?;
    self.player_x = wilderness.width / 2;
    self.player_y = wilderness.height / 2;
    self.camera.center_on(self.player_x, self.player_y);

    Ok(())
  }

  pub fn exit_wilderness(&mut self) -> Result<()>{
    match self.view_mode{
      ViewMode::Wilderness(wilderness_entity) => {
        let wilderness_location = self.entity_manager.get_component::<Location, _>(wilderness_entity)?;
        let parent_location_id = wilderness_location.get().parent_location_id
          .ok_or_else(|| anyhow!("no parent location found for wilderness"))?;
        let hex_entity = self.entity_manager.find_entity_by_location_id::<Hex>(parent_location_id)
          .ok_or_else(|| anyhow!("no hex found at parent location"))?;
        let hex_spatial = self.entity_manager.get_component::<Spatial, _>(hex_entity)?;
        let hex_spatial_data = hex_spatial.get();

        self.player_x = hex_spatial_data.get_x();
        self.player_y = hex_spatial_data.get_y();

        self.view_mode = ViewMode::HexMap;
        self.camera.center_on(self.player_x, self.player_y);
        Ok(())
      }
      _ => {
        Err(anyhow!("not currently in wilderness mode"))
      }
    }
  }

  pub fn get_wilderness_tile(&self, x: i32, y: i32) -> Option<Tile>{
    if let ViewMode::Wilderness(wilderness_entity) = self.view_mode{
      if let Some(wilderness) = self.wilderness_cache.get(&wilderness_entity){
        if let Some(wtile) = wilderness.get(x, y){
          return Some(wtile.tile)
        }
      }
    }
    None
  }

  pub fn get_dungeon_tile(&self, x: i32, y: i32) -> Option<Tile>{
    if let ViewMode::Dungeon(dungeon_entity) = self.view_mode{
      if let Some(dungeon) = self.dungeon_cache.get(&dungeon_entity){
        if let Some(tile) = dungeon.get_tile(x, y){
          return Some(*tile)
        }
      }
    }
    None
  }

  pub async fn generate_hex_at(&mut self, x: i32, y: i32) -> Result<()>{
    info!("generating hex at {}, {}", x, y);
    let prev = self.map.get((x-1, y))
      .and_then(|entity| self.entity_manager.get_component::<Hex, _>(entity).ok());

    info!("previous hex retrieved");
    info!("prev hex: {:?}", prev);
    let entity = self.entity_manager.create_entity(
      EntityKind::Hex,
      json!({
        "x": x,
        "y": y,
        "prev": prev
      })
    ).await?;
    info!("hex created");

    self.map.insert((x, y), entity);
    info!("hex inserted into map");

    if let Ok(hex) = self.entity_manager.get_component::<Hex, _>(entity){
      let tile = Tile::from_terrain_type(&hex.get());
      let mut world = self.entity_manager.world.lock().unwrap();
      world.insert_one(entity, tile).ok();
    }
    info!("hex tile inserted into world");

    Ok(())
  }

  pub async fn generate_dungeon(&mut self) -> Result<()>{
    info!("generating dungeon");
    let dungeon = self.entity_manager.create_entity(
      EntityKind::Dungeon,
      json!({
        "seed": 0,
        "depth_levels": 1,
        "x": 0,
        "y": 0,
      })
    ).await?;

    info!("dungeon {:?} generated", dungeon);

    self.enter_dungeon(dungeon).await?;

    Ok(())
  }

  pub fn get_current_hex(&self) -> Result<Hex>{
    let hex_entity = self.entity_manager.find_entity_at::<Hex>(self.player_x, self.player_y);
    match hex_entity{
      Some(entity) => self.entity_manager.get_component::<Hex, _>(entity),
      None => Err(anyhow!("unable to find current hex at player location")),
    }
  }

  pub fn rebuild_map(&mut self){
    self.map.clear();
    self.entity_manager.for_each::<(&Hex, &Spatial), _>(|entity, (_hex, spatial) |{
      self.map.insert((spatial.get().x, spatial.get().y), entity);
    });
  }

  pub fn attach_tiles(&mut self){
    let mut tiles = Vec::new();
    self.entity_manager.for_each::<&Hex, _>(|entity, hex|{
      tiles.push((entity, Tile::from_terrain_type(&hex.get())));
    });

    let mut world = self.entity_manager.world.lock().unwrap();
    for (entity, tile) in tiles{
      world.insert_one(entity, tile).ok();
    }
  }

  pub async fn ascend(&mut self) -> Result<()>{
    match self.view_mode{
      ViewMode::HexMap => {
        info!("already at top level");
      },
      ViewMode::Wilderness(_wilderness_entity) => {
        self.exit_wilderness()?;
      },
      ViewMode::Dungeon(dungeon_entity) => {
        let current_level = self.dungeon_cache
          .iter()
          .find(|((e, _), area)| *e == dungeon_entity &&
            self.dungeon_cache.get(&(*e, area.current_level)).is_some())
          .map(|((_, _), area)| area.current_level)
          .unwrap_or(1);

        let dungeon_area = self.dungeon_cache.get(&(dungeon_entity, current_level))
          .ok_or_else(|| anyhow!("no dungeon found at hex location"))?;

        if !dungeon_area.is_stairs_up(self.player_x, self.player_y){
          info!("not standing on dungeon stairs up");
          return Ok(());
        }

        if current_level == 1{
          let dungeon_location = self.entity_manager.get_component::<Location, _>(dungeon_entity)?;
          let parent_hex_id = dungeon_location.get().get_parent_location_id();

          if let Some(hex_id) = parent_hex_id{
            if let Some(wilderness_entity) = self.entity_manager.find_child_entity::<Wilderness>(hex_id){
              if let Some(wilderness) = self.wilderness_cache.get(&wilderness_entity){
                if let Some((entrance_x, entrance_y)) = wilderness.dungeon_entrance{
                  self.player_x = entrance_x;
                  self.player_y = entrance_y;
                }
              }
              self.view_mode = ViewMode::Wilderness(wilderness_entity);
              self.camera.center_on(self.player_x, self.player_y);
              return Ok(())
            }
          }

          self.exit_dungeon()?;
        }else{
          let dungeon_location = self.entity_manager.get_component::<Location, _>(dungeon_entity)?;
          let seed = dungeon_location.get().get_seed().unwrap_or(0);

          let new_level = current_level - 1;
          info!("ascending to dungeon level {}", new_level);

          if !self.dungeon_cache.contains_key(&(dungeon_entity, new_level)){
            let generator = DungeonGenerator::new(seed as u64);
            let new_area = generator.generate(dungeon_entity, &self.entity_manager, new_level)?;
            self.dungeon_cache.insert((dungeon_entity, new_level), new_area);
          }

          let new_area = self.dungeon_cache.get(&(dungeon_entity, new_level))
            .ok_or_else(|| anyhow!("failed to get new level"))?;

          if let Some((down_x, down_y)) = new_area.stairs_down{
            self.player_x = down_x;
            self.player_y = down_y
          }

          self.camera.center_on(self.player_x, self.player_y);
          self.visibility.clear()
        }
      },
    }

    Ok(())
  }

  pub async fn descend(&mut self) -> Result<()>{
    match self.view_mode{
      ViewMode::HexMap => {
        self.enter_wilderness().await?;
      },
      ViewMode::Wilderness(wilderness_entity) => {
        let wilderness = self.wilderness_cache.get(&wilderness_entity)
          .ok_or_else(|| anyhow!("no wilderness found at hex location"))?;

        if !wilderness.is_dungeon_entrance(self.player_x, self.player_y){
          info!("not standing on dungeon entrance");
          return Ok(());
        }

        let wilderness_location = self.entity_manager.get_component::<Location, _>(wilderness_entity)?;
        let parent_hex_id = wilderness_location.get().get_parent_location_id()
          .ok_or_else(|| anyhow!("no parent location found for wilderness"))?;

        if let Some(dungeon_entity) = self.entity_manager.find_child_entity::<Dungeon>(parent_hex_id){
          self.enter_dungeon(dungeon_entity).await?;
        }else{
          info!("no dungeon found at parent hex location");
        }
      },
      ViewMode::Dungeon(dungeon_entity) => {
        let current_level = self.dungeon_cache
          .iter()
          .find(|((e, _), _)| *e == dungeon_entity)
          .and_then(|((_, _), area)| Some(area.current_level))
          .unwrap_or(1);

        let dungeon_area = self.dungeon_cache.get(&(dungeon_entity, current_level))
          .ok_or_else(|| anyhow!("no dungeon found in cache"))?;

        if !dungeon_area.is_stairs_down(self.player_x, self.player_y){
          info!("not standing on dungeon stairs down");
          return Ok(())
        }

        let dungeon_location = self.entity_manager.get_component::<Location, _>(dungeon_entity)?;
        let seed = dungeon_location.get().get_seed().unwrap_or(0);

        let new_level = current_level + 1;
        info!("descending to dungeon level {}", new_level);

        if !self.dungeon_cache.contains_key(&(dungeon_entity, new_level)){
          let generator = DungeonGenerator::new(seed as u64);
          let new_area = generator.generate(dungeon_entity, &self.entity_manager, new_level)?;
          self.dungeon_cache.insert((dungeon_entity, new_level), new_area);
        }

        let new_area = self.dungeon_cache.get(&(dungeon_entity, new_level))
          .ok_or_else(|| anyhow!("failed to get new level"))?;

        if let Some((up_x, up_y)) = new_area.stairs_up{
          self.player_x = up_x;
          self.player_y = up_y;
        }

        self.camera.center_on(self.player_x, self.player_y);
        self.visibility.clear();
      },
    }

    Ok(())
  }

  pub async fn enter_dungeon(&mut self, dungeon_entity: Entity) -> Result<()>{
    info!("entering dungeon");
    let dungeon_generator = DungeonGenerator::new(0);
    let level = 1;

    if !self.dungeon_cache.contains_key(&(dungeon_entity, level)){
      info!("dungeon level {} not cached, generating area", level);
      let dungeon_area = dungeon_generator.generate(dungeon_entity, &self.entity_manager, level)?;
      debug!("dungeon area: {:?}", dungeon_area);
      self.dungeon_cache.insert((dungeon_entity, level), dungeon_area);
    }

    self.view_mode = ViewMode::Dungeon(dungeon_entity);

    let dungeon = self.dungeon_cache.get(&(dungeon_entity, level))
      .ok_or_else(|| anyhow!("no dungeon found at hex location"))?;

    if let Some((entrance_x, entrance_y)) = dungeon.entrance{
      self.player_x = entrance_x;
      self.player_y = entrance_y;
    }
    self.camera.center_on(self.player_x, self.player_y);
    self.visibility.clear();

    Ok(())
  }

  pub fn exit_dungeon(&mut self) -> Result<()>{
    match self.view_mode{
      ViewMode::Dungeon(dungeon_entity) => {
        let dungeon_location = self.entity_manager.get_component::<Location, _>(dungeon_entity)?;
        let parent_location_id = dungeon_location.get().parent_location_id;

        if let Some(parent_id) = parent_location_id{
          let hex_entity = self.entity_manager.find_entity_by_location_id::<Hex>(parent_id)
            .ok_or_else(|| anyhow!("no hex found at parent location"))?;
          let hex_spatial = self.entity_manager.get_component::<Spatial, _>(hex_entity)?;
          let hex_spatial_data = hex_spatial.get();

          self.player_x = hex_spatial_data.get_x();
          self.player_y = hex_spatial_data.get_y();
        }else{
          self.player_x = 0;
          self.player_y = 0;
        }

        self.view_mode = ViewMode::HexMap;
        self.camera.center_on(self.player_x, self.player_y);
        Ok(())
      }
      _ => Err(anyhow!("not currently in dungeon mode")),
    }
  }
}
