use crossterm::style::Color;
use dagr_lib::components::world::hex::HexData;
use dagr_lib::kits::hexkit::terrain::Terrain;
use crate::visiblity::Visibility;

#[derive(Clone, Debug)]
pub struct RenderConfig{
  pub unseen_symbol: char,
  pub unseen_fg: Color,
  pub unseen_bg: Colir,
  pub player_symbol: char,
  pub player_fg: Color,
  pub player_bg: Color,
  pub dim_factor: f32,
}

impl Default for RenderConfig{
  fn default() -> Self{
    self{
      unseen_symbol: ' ',
      unseen_fg: Color::Black,
      unseen_bg: Color::Black,
      player_symbol: '@',
      player_fg: Color::White,
      player_bg: Color::Blue,
      dim_factor: 0.4,
    }
  }
}

impl RenderConfig{
  pub fn dim_color(&self, color: Color) -> Color{
    match color{
      Color::Black => Color::Black,
      Color::White => Color::DarkGrey,
      Color::Grey => Color::DarkGrey,
    }
  }
}
