use std::io::Write;
use anyhow::Result;
use crossterm::style::Color;
use crate::game_state::GameState;
use crate::ui::{map::Map, monster_picker, stat_bar::StatBar, panel::Panel, popup::Popup};

pub mod render_config;
pub use render_config::RenderConfig;
pub mod tile;
pub use tile::Tile;

pub struct Renderer{
  width: u16,
  height: u16,
  map_height: u16,
}

impl Renderer{
  pub fn new(width: u16, height: u16) -> Self{
    Self{width, height, map_height: height*80/100}
  }

  pub fn render(&self, stdout: &mut std::io::Stdout, game_state: &GameState) -> Result<()>{
    let map = Map::new(0, 0, self.width, self.map_height);

    if game_state.is_in_world(){
      self.render_hexmap(stdout, &map, game_state)?;
    }else{
      self.render_location(stdout, &map, game_state)?;
    }

    self.render_ui(stdout, game_state)?;

    if let Some(ref message) = game_state.popup_message{
      let popup = Popup::new(message.clone(), self.width, self.height);
      popup.draw(stdout)?;
    }
    if let Some(picker) = game_state.combat.picker(){
      monster_picker::draw(stdout,picker,self.width,self.height)?;
    }
    
    stdout.flush()?;
    Ok(())
  }

  fn render_hexmap(&self, stdout: &mut std::io::Stdout, map: &Map, game_state: &GameState) -> Result<()>{
    map.draw(stdout, |x, y|{
      let world_x = x as i32 + game_state.camera.x;
      let world_y = y as i32 + game_state.camera.y;

      if world_x == game_state.player_x && world_y == game_state.player_y{
        return Some(game_state.render_config.player_tile());
      }

      match game_state.map.get((world_x, world_y)){
        Some(entity) => {
          game_state.entity_manager.with::<Tile, _, _>(
            entity,
            |tile| *tile
          ).ok()
        }
        None => {
          Some(Tile::new('░', Color::DarkGrey))
        }
      }
    })?;
    Ok(())
  }

  fn render_location(&self, stdout: &mut std::io::Stdout, map: &Map, game_state: &GameState) -> Result<()>{
    map.draw(stdout, |x, y|{
      let world_x = x as i32 + game_state.camera.x;
      let world_y = y as i32 + game_state.camera.y;

      if world_x == game_state.player_x && world_y == game_state.player_y{
        return Some(game_state.render_config.player_tile());
      }

      game_state.get_location_tile(world_x, world_y)
    })?;

    Ok(())
  }

  fn render_ui(&self, stdout: &mut std::io::Stdout, game_state: &GameState) -> Result<()>{
    let (current_hp,max_hp)=game_state.player_hit_points().unwrap_or((0,0));
    let stat_bar = StatBar::new(
      1,self.map_height,"VITALITY".to_string(),current_hp.max(0) as u32,max_hp.max(0) as u32,20,
    );
    stat_bar.draw(stdout)?;

    let hexes_explored = game_state.map.count();
    let mut stats = game_state.coordinate_debug_lines();
    stats.extend([
      format!("explored: {}", hexes_explored),
      format!("view: {}", game_state.current_view_label()),
      format!("cam: ({}, {})", game_state.camera.x, game_state.camera.y),
    ]);
    let mut stats_panel = Panel::new(35, self.map_height, 28, 8);
    stats_panel.set_title("Stats".to_string());
    stats_panel.set_content(stats);
    stats_panel.draw(stdout)?;

    let controls_width = self.width.saturating_sub(63);
    if controls_width >= 20 {
      let mut controls = Panel::new(63, self.map_height, controls_width, 8);
      controls.set_title("Controls".to_string());
      controls.set_content(vec![
        "move: arrows / hjklyubn".to_string(),
        "enter/descend: >  ascend: <".to_string(),
        "auto-explore: o or O".to_string(),
        "dismiss popup: Space".to_string(),
        "spawn: M  reset arena: R".to_string(),
        "quit: q".to_string(),
      ]);
      controls.draw(stdout)?;
    }

    // let hex_data = game_state.get_current_hex()?.get();
    // let hex = Tile::from_terrain_type(&hex_data);
    // let mut hex_panel = Panel::new(42, self.map_height, 30, 8);
    // hex_panel.set_title("Hex".to_string());
    // let hex_content = vec![
    //   format!("{}", hex.symbol),
    //   format!("terrain: {}", hex_data.get_terrain()),
    //   format!("vegetation: {}", hex_data.get_vegetation()),
    //   format!("water: {}", hex_data.get_water()),
    // ];
    // hex_panel.set_content(hex_content);
    // hex_panel.draw(stdout)?;
    Ok(())
  }
}
