use std::collections::HashMap;
use anyhow::{anyhow, Result};
use hecs::Entity;
use serde_json::json;
use dagr_lib::components::world::{
  hex::Hex,
  spatial::Spatial,
  wilderness::Wilderness
};
use dagr_lib::components::world::hex::HexData;
use dagr_lib::core::registry::EntityKind;
use dagr_lib::ems::{entity_manager::EntityManager, component::Component};
use crate::camera::Camera;
use crate::tile::Tile;
use crate::wilderness_generator::{WildernessArea, WildernessGenerator};
use crate::world_map::WorldMap;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ViewMode{
  HexMap,
  Wilderness(Entity),
}

pub struct GameState{
  pub entity_manager: EntityManager,
  pub map: WorldMap,
  pub camera: Camera,
  pub player_x: i32,
  pub player_y: i32,
  pub view_mode: ViewMode,
  wilderness_cache: HashMap<Entity, WildernessArea>,
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
    };
    state.rebuild_map();
    state.attach_tiles();
    state
  }

  pub async fn move_player(&mut self, dx: i32, dy: i32) -> Result<()>{
    let new_x = self.player_x + dx;
    let new_y = self.player_y + dy;
    let mut can_move = true;

    match self.view_mode{
      ViewMode::HexMap => {
        if self.map.get((new_x, new_y)).is_none(){
          self.generate_hex_at(new_x, new_y).await?;
        }
      }
      ViewMode::Wilderness(hex_entity) => {
        can_move = false;
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

    Ok(())
  }

  pub async fn generate_hex_at(&mut self, x: i32, y: i32) -> Result<()>{
    let prev = self.map.get((x-1, y))
      .and_then(|entity| self.entity_manager.get_component::<Hex, _>(entity).ok());

    let entity = self.entity_manager.create_entity(
      EntityKind::Hex,
      json!({
        "x": x,
        "y": y,
        "prev": prev
      })
    ).await?;

    self.map.insert((x, y), entity);

    if let Ok(hex) = self.entity_manager.get_component::<Hex, _>(entity){
      let tile = Tile::from_terrain_type(&hex);
      let mut world = self.entity_manager.world.lock().unwrap();
      world.insert_one(entity, tile).ok();
    }

    Ok(())
  }

  pub fn get_current_hex(&self) -> Result<HexData>{
    let hex_entity = self.entity_manager.find_entity_at::<Hex>(self.player_x, self.player_y);
    match hex_entity{
      Some(entity) => self.entity_manager.get_component::<Hex, _>(entity),
      None => Err(anyhow!("No hex found at player position")),
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
}
