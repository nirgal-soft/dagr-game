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
    tile: Tile{symbol: '.', fg: Color::Green, bg: Color::Black}
  };
  pub const MOUNTAIN: Self = Self{
    tile: Tile{symbol: '.', fg: Color::Grey, bg: Color::Rgb{r: 18, g: 20, b: 18}}
  };
  pub const HILLS: Self = Self{
    tile: Tile{symbol: '.', fg: Color::DarkYellow, bg: Color::Black}
  };
  pub const PLAINS: Self = Self{
    tile: Tile{symbol: ',', fg: Color::DarkGreen, bg: Color::Black}
  };
  pub const SWAMP: Self = Self{
    tile: Tile{symbol: '.', fg: Color::DarkCyan, bg: Color::Rgb{r: 8, g: 18, b: 16}}
  };
}
