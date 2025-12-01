use crossterm::style::Color;
use dagr_lib::components::world::hex::HexData;
use dagr_lib::kits::hexkit::terrain::Terrain;
use crate::visiblity::Visibility;
use super::render_config::RenderConfig;

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

  pub fn with_visibility(&self, visibility: Visibility, config: &RenderConfig) -> Tile{
    match visibility{
      Visibility::Unseen => Tile{
        symbol: config.unseen_symbol,
        fg: config.unseen_fg,
        bg: config.unseen_bg,
      },
      Visibility::Seen => Tile{
        symbol: self.symbol,
        fg: config.dim_color(self.fg),
        bg: config.dim_color(self.bg),
      },
      Visibility::Visible => *self,
    }
  }

  pub fn from_terrain_type(hex_data: &HexData) -> Self{
    match hex_data.get_terrain(){
      Terrain::Mountains => Tile::new('^', Color::White).with_bg(Color::DarkGrey),
      Terrain::Hills => Tile::new('n', Color::Yellow).with_bg(Color::Rgb{r: 60, g: 50, b: 30}),
      Terrain::Plains => Tile::new('~', Color::Green).with_bg(Color::Rgb{r: 20, g: 60, b: 20}),
      Terrain::Swamp => Tile::new('"', Color::DarkYellow).with_bg(Color::Rgb{r: 30, g: 50, b: 30}),
    }
  }
}
