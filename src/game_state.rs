use std::collections::HashMap;
use anyhow::{anyhow, Context, Result};
use crossterm::style::Color;
use hecs::Entity;
use serde_json::json;
use tracing::{debug, error, info};
use dagr_lib::components::world::{
  hex::{Hex, HexData},
  location::Location,
  spatial::Spatial,
  wilderness::Wilderness
};
use dagr_lib::core::registry::EntityKind;
use dagr_lib::ems::{entity_manager::EntityManager, component::Component};
use crate::camera::Camera;
use crate::dungeon_generator::{DungeonArea, DungeonGenerator};
use crate::tile::Tile;
use crate::wilderness_generator::{WildernessArea, WildernessGenerator, wilderness_tile::WildernessTile};
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
  wilderness_cache: HashMap<Entity, WildernessArea>,
  dungeon_cache: HashMap<Entity, DungeonArea>,
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
      wilderness_cache: HashMap::new(),
      dungeon_cache: HashMap::new(),
    };
    state.rebuild_map();
    state.attach_tiles();
    state
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
        if let Some(dungeon) = self.dungeon_cache.get(&dungeon_entity){
          if !dungeon.contains(new_x, new_y){
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

    let spatial = self.entity_manager.get_component::<Spatial, _>(wilderness_entity)?;
    let wilderness = self.entity_manager.get_component::<Wilderness, _>(wilderness_entity)?;
    info!("wilderness entity: {:?}", wilderness_entity);
    info!("wilderness component: {:?}", wilderness);

    if !self.wilderness_cache.contains_key(&wilderness_entity){
      info!("wilderness not cached, generating");
      let seed = hex_location.get().get_seed().unwrap_or(0);
      let spatial_data = spatial.get();
      let generator = WildernessGenerator::new(seed as u64);
      let area = generator.generate(spatial_data.get_width(), spatial_data.get_length())?;
      self.wilderness_cache.insert(wilderness_entity, area);
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

  pub fn get_wilderness_tile(&self, x: i32, y: i32) -> Option<(char, Color)>{
    if let ViewMode::Wilderness(wilderness_entity) = self.view_mode{
      if let Some(wilderness) = self.wilderness_cache.get(&wilderness_entity){
        if let Some(wtile) = wilderness.get(x, y){
          return Some((wtile.tile.symbol, wtile.tile.color))
        }
      }
    }
    None
  }

  pub fn get_dungeon_tile(&self, x: i32, y: i32) -> Option<(char, Color)>{
    if let ViewMode::Dungeon(dungeon_entity) = self.view_mode{
      if let Some(dungeon) = self.dungeon_cache.get(&dungeon_entity){
        if let Some(tile) = dungeon.get_tile(x, y){
          return Some((tile.symbol, tile.color))
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

  pub async fn enter_dungeon(&mut self, dungeon_entity: Entity) -> Result<()>{
    info!("entering dungeon");
    let dungeon_generator = DungeonGenerator::new(0);

    if !self.dungeon_cache.contains_key(&dungeon_entity){
      info!("dungeon not cached, building area");
      // let dungeon_area = dungeon_generator.generate(dungeon_entity, &self.entity_manager)?;
      let dungeon_area = dungeon_generator.generate_raw(45, 45, 5, 4)?;
      debug!("dungeon area: {:?}", dungeon_area);
      self.dungeon_cache.insert(dungeon_entity, dungeon_area);
    }

    self.view_mode = ViewMode::Dungeon(dungeon_entity);

    let dungeon = self.dungeon_cache.get(&dungeon_entity)
      .ok_or_else(|| anyhow!("no dungeon found at hex location"))?;

    self.player_x = dungeon.width / 2;
    self.player_y = dungeon.height / 2;
    self.camera.center_on(self.player_x, self.player_y);

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
