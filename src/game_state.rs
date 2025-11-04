use std::sync::{Arc, Mutex};
use hecs::World;
use dagr_lib::components::world::{
  hex::Hex,
  spatial::Spatial
};
use dagr_lib::ems::{entity_manager::EntityManager, component::Component};
use crate::map::Map;
use crate::tile::Tile;

pub struct GameState{
  pub entity_manager: EntityManager,
  pub map: Map,
  pub camera_x: i32,
  pub camera_y: i32,
}

impl GameState{
  pub fn new(entity_manager: EntityManager) -> Self{
    let mut state = Self{
      entity_manager,
      map: Map::new(),
      camera_x: 0,
      camera_y: 0,
    };
    state.rebuild_map();
    state.attach_tiles();
    state
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
