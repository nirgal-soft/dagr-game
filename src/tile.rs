use crossterm::style::Color;

pub struct Tile{
  pub symbol: char,
  pub color: Color,
}

impl Tile{
  pub fn new(symbol: char, color: Color) -> Self{
    Self{
      symbol,
      color,
    }
  }
}
