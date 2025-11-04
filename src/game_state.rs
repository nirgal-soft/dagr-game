use std::sync::{Arc, Mutex};
use hecs::World;
use dagr_lib::components::world::{
  hex::HexData,
  spatial::SpatialData
};
use crate::map::Map;

pub struct GameState{
  pub world: Arc<Mutex<World>>,
  pub map: Map,
  pub camera_x: i32,
  pub camera_y: i32,
}

impl GameState{
  pub fn new(world: Arc<Mutex<World>>) -> Self{
    let mut state = Self{
      world,
      map: Map::new(),
      camera_x: 0,
      camera_y: 0,
    };
    state.rebuild_map();
    state
  }

  pub fn rebuild_map(&mut self){
    self.map.clear();
    let world = self.world.lock().unwrap();

    for(entity, (_hex, spatial)) in world.query::<(&HexData, &SpatialData)>().iter(){
      self.map.insert((spatial.get_x(), spatial.get_y()), entity);
    }
  }
}
