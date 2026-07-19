use crossterm::style::Color;
use dagr_lib::components::world::hex::HexData;
use dagr_lib::kits::hexkit::terrain::Terrain;
use crate::visiblity::Visibility;
use super::render_config::RenderConfig;

#[derive(Clone, Copy, Debug, PartialEq)]
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

  pub fn inverted(mut self)->Self{
    std::mem::swap(&mut self.fg,&mut self.bg);
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
      Ok(Terrain::Mountains) => Tile::new('^', Color::White).with_bg(Color::DarkGrey),
      Ok(Terrain::Hills) => Tile::new('n', Color::Yellow).with_bg(Color::Rgb{r: 60, g: 50, b: 30}),
      Ok(Terrain::Plains) => Tile::new('~', Color::Green).with_bg(Color::Rgb{r: 20, g: 60, b: 20}),
      Ok(Terrain::Swamp) => Tile::new('"', Color::DarkYellow).with_bg(Color::Rgb{r: 30, g: 50, b: 30}),
      Err(_) => Tile::new('?', Color::Magenta),
    }
  }
}

#[cfg(test)]
mod tests{
  use super::*;

  #[test]
  fn inverted_tile_swaps_foreground_and_background(){
    let tile=Tile::new('=',Color::Yellow).with_bg(Color::Blue).inverted();
    assert_eq!(tile.fg,Color::Blue);
    assert_eq!(tile.bg,Color::Yellow);
    assert_eq!(tile.symbol,'=');
  }
}
