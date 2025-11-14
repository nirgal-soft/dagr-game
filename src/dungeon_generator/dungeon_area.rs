use std::collections::HashMap;
use crate::tile::Tile;

#[derive(Debug)]
pub struct DungeonArea{
  pub width: i32,
  pub height: i32,
  tiles: HashMap<(i32, i32), Tile>,
}

impl DungeonArea{
  pub fn new(width: i32, height: i32) -> Self{
    Self{
      width,
      height,
      tiles: HashMap::new(),
    }
  }

  pub fn set_tile(&mut self, x: i32, y: i32, tile: Tile){
    self.tiles.insert((x, y), tile);
  }

  pub fn get_tile(&self, x: i32, y: i32) -> Option<&Tile>{
    self.tiles.get(&(x, y))
  }

  pub fn contains(&self, x: i32, y: i32) -> bool{
    x >= 0 && x < self.width && y >= 0 && y < self.height
  }
}
