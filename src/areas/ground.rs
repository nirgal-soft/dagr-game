use crossterm::style::Color;
use crate::renderer::Tile;

#[derive(Clone, Copy, Debug)]
pub struct Ground{
  pub tile: Tile,
}

impl Ground{
  pub const DUNGEON: Self = Self{
    tile: Tile{
      symbol: '.',
      fg: Color::Grey,
      bg: Color::Rgb{r: 20, g: 20, b: 25},
    }
  };

  pub const WILDERNESS: Self = Self{
    tile: Tile{
      symbol: '.',
      fg: Color::Green,
      bg: Color::Black,
    }
  };
}
