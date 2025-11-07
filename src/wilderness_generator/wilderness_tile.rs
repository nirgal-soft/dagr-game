use crossterm::style::Color;
use crate::tile::Tile;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum WildernessTileType{
  Grass,
  Tree,
  Water,
  Rock,
  Dirt,
}

#[derive(Clone, Copy, Debug)]
pub struct WildernessTile{
  pub tile_type: WildernessTileType,
  pub tile: Tile,
}

impl WildernessTile{
  pub fn new(tile_type: WildernessTileType) -> Self{
    let (symbol, color) = match tile_type{
      WildernessTileType::Grass => ('.', Color::Green),
      WildernessTileType::Tree => ('T', Color::DarkGreen),
      WildernessTileType::Water => ('~', Color::Blue),
      WildernessTileType::Rock => ('o', Color::Grey),
      WildernessTileType::Dirt => (',', Color::Yellow),
    };
    let tile = Tile::new(symbol, color);

    Self{
      tile_type,
      tile,
    }
  }
}
