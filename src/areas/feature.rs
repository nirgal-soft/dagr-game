use crossterm::style::Color;
use crate::renderer::Tile;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Feature{
  pub tile: Tile,
  pub walkable: bool,
  pub opaque: bool,
}

impl Feature{
  pub const fn new(symbol: char, fg: Color, walkable: bool, opaque: bool) -> Self{
    Self{
      tile: Tile{symbol, fg, bg: Color::Black},
      walkable,
      opaque,
    }
  }

  pub const fn with_bg(mut self, bg: Color) -> Self{
    self.tile.bg = bg;
    self
  }

  //common features, TODO abstract these later
  pub const WALL: Self = Self::new('#', Color::White, false, true)
    .with_bg(Color::Rgb{r: 40, g: 35, b: 30});
  pub const TREE: Self = Self::new('T', Color::DarkGreen, false, true);
  pub const ROCK: Self = Self::new('o', Color::Grey, false, true);
  pub const WATER: Self = Self::new('~', Color::Blue, false, false);
  pub const STAIRS_UP: Self = Self::new('<', Color::Yellow, false, false)
    .with_bg(Color::Rgb{r: 50, g: 40, b: 20});
  pub const STAIRS_DOWN: Self = Self::new('>', Color::Yellow, false, false)
    .with_bg(Color::Rgb{r: 50, g: 40, b: 20});
  pub const DOOR_CLOSED: Self = Self::new('+', Color::White, false, true);
  pub const DOOR_OPEN: Self = Self::new('/', Color::White, false, false);

  pub fn is_stairs_up(&self) -> bool{
    self.tile.symbol == '<'
  }

  pub fn is_stairs_down(&self) -> bool{
    self.tile.symbol == '>'
  }
}
