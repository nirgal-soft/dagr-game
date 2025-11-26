use std::collections::HashMap;
use crate::tile::Tile;

pub enum DungeonTileType{
  Floor,
  Wall,
}

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

  pub fn set_tile(&mut self, x: i32, y: i32, tile_type: DungeonTileType){
    let tile = match tile_type{
      DungeonTileType::Floor => Tile{symbol: '.', color: crossterm::style::Color::Grey},
      DungeonTileType::Wall => Tile{symbol: '#', color: crossterm::style::Color::White},
    };
    self.tiles.insert((x, y), tile);
  }

  pub fn get_tile(&self, x: i32, y: i32) -> Option<&Tile>{
    self.tiles.get(&(x, y))
  }

  pub fn contains(&self, x: i32, y: i32) -> bool{
    x >= 0 && x < self.width && y >= 0 && y < self.height
  }

  pub fn is_walkable(&self, x: i32, y: i32) -> bool{
    if let Some(tile) = self.tiles.get(&(x, y)){
      tile.symbol == '.'
    }else{
      false
    }
  }
}
