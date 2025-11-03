use crate::tile::Tile;

pub struct Object{
  pub tile: Tile,
  pub x: u16,
  pub y: u16,
}

impl Object{
  pub fn new(tile: Tile, x: u16, y: u16) -> Self{
    Self{
      tile,
      x,
      y,
    }
  }
}
