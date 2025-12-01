use crossterm::style::Color;
use dagr_lib::components::world::hex::HexData;
use dagr_lib::kits::hexkit::terrain::Terrain;
use crate::visiblity::Visibility;

#[derive(Clone, Copy, Debug)]
pub struct Tile{
  pub symbol: char,
  pub fg: Color,
  pub bg: Color,
}

impl Tile{
  pub fn new(symbol: char, fg: Color) -> Self{
    Self{
      symbol,
      fg,
      bg: Color::Black,
    }
  }

  pub fn with_bg(mut self, bg: Color) -> Self{
    self.bg = bg;
    self
  }

  pub fn with_visibility(&self, visiblity: Visibility, config: &RenderConfig) -> TileAppearance{

  }

  pub fn from_terrain_type(hex_data: &HexData) -> Self{
    match hex_data.get_terrain(){
      Terrain::Mountains => Tile{symbol: '^', color: Color::DarkGrey},
      Terrain::Hills => Tile{symbol: 'n', color: Color::Grey},
      Terrain::Plains => Tile{symbol: '~', color: Color::Green},
      Terrain::Swamp => Tile{symbol: '"', color: Color::DarkYellow},
      _ => Tile{symbol: '?', color: Color::White},
    }
  }
}
