use super::wilderness_tile::{WildernessTile, WildernessTileType};

pub struct WildernessArea{
  pub width: i32,
  pub height: i32,
  pub tiles: Vec<Vec<WildernessTile>>,
  pub dungeon_entrance: Option<(i32, i32)>,
}

impl WildernessArea{
  pub fn get(&self, x: i32, y: i32) -> Option<&WildernessTile>{
    if x >= 0 && x < self.width && y >= 0 && y < self.height{
      self.tiles.get(y as usize)?.get(x as usize)
    }else{
      None
    }
  }

  pub fn set(&mut self, x: i32, y: i32, tile: WildernessTile){
    if x >= 0 && x < self.width && y >= 0 && y < self.height{
      self.tiles[y as usize][x as usize] = tile;
    }
  }

  pub fn set_dungeon_entrance(&mut self, x: i32, y: i32){
    self.dungeon_entrance = Some((x, y));
    self.set(x, y, WildernessTile::new(WildernessTileType::StairsDown));
  }

  pub fn is_dungeon_entrance(&self, x: i32, y: i32) -> bool{
    if let Some(tile) = self.get(x, y){
      tile.tile_type == WildernessTileType::StairsDown
    }else{
      false
    }
  }

  pub fn contains(&self, x: i32, y: i32) -> bool{
    x >= 0 && x < self.width && y >= 0 && y < self.height
  }
}
