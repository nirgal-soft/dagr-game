use std::io::Write;
use anyhow::Result;
use crossterm::{queue, cursor, style::{self, Stylize, Color}};
use crate::tile::Tile;
use crate::game_state::GameState;
use crate::ui::{panel::Panel, stat_bar::StatBar, map::Map};

pub struct Renderer{
  width: u16,
  height: u16,
  map_height: u16,
}

impl Renderer{
  pub fn new(width: u16, height: u16) -> Self{
    Self{width, height, map_height: height/2}
  }

  pub fn render(&self, stdout: &mut std::io::Stdout, game_state: &GameState) -> Result<()>{
    let map = Map::new(0, 0, self.width, self.map_height);

    map.draw(stdout, |x, y|{
      let world_x = x as i32 + game_state.camera.x;
      let world_y = y as i32 + game_state.camera.y;

      if world_x == game_state.player_x && world_y == game_state.player_y{
        return Some(('@', Color::Blue));
      }

      match game_state.map.get((world_x, world_y)){
        Some(entity) => {
          game_state.entity_manager.with::<Tile, _, _>(
            entity,
            |tile| (tile.symbol, tile.color)
          ).ok()
        }
        None => {
          Some(('░', Color::DarkGrey))
        }
      }
    })?;

    let stat_bar = StatBar::new(1, self.map_height, "HP".to_string(), 45, 100, 20);
    stat_bar.draw(stdout)?;

    let hexes_explored = game_state.map.count();
    let stats = vec![
      format!("pos: ({}, {})", game_state.player_x, game_state.player_y),
      format!("cam: ({}, {})", game_state.camera.x, game_state.camera.y),
      format!("explored: {}", hexes_explored),
      "STR: 14".to_string(),
      "DEX: 16".to_string(),
      "CON: 12".to_string(),
    ];
    let mut stats_panel = Panel::new(21, self.map_height, 20, 8);
    stats_panel.set_title("Stats".to_string());
    stats_panel.set_content(stats);
    stats_panel.draw(stdout)?;

    let hex_data = game_state.get_current_hex()?;
    let hex = Tile::from_terrain_type(&hex_data);
    let mut hex_panel = Panel::new(42, self.map_height, 30, 8);
    hex_panel.set_title("Hex".to_string());
    let hex_content = vec![
      format!("{}", hex.symbol),
      format!("terrain: {}", hex_data.get_terrain()),
      format!("vegetation: {}", hex_data.get_vegetation()),
      format!("water: {}", hex_data.get_water()),
    ];
    hex_panel.set_content(hex_content);
    hex_panel.draw(stdout)?;
    stdout.flush()?;
    Ok(())
  }
}
