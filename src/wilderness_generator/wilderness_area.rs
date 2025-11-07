use super::wilderness_tile::WildernessTile;

pub struct WildernessArea{
  pub width: i32,
  pub height: i32,
  pub tiles: Vec<Vec<WildernessTile>>
}

impl WildernessArea{
  pub fn get(&self, x: i32, y: i32) -> Option<&WildernessTile>{
    if x >= 0 && x < self.width && y >= 0 && y < self.height{
      self.tiles.get(y as usize)?.get(x as usize)
    }else{
      None
    }
  }

  pub fn contains(&self, x: i32, y: i32) -> bool{
    x >= 0 && x < self.width && y >= 0 && y < self.height
  }
}
