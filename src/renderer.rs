use std::io::{Write, self};
use std::sync::{Arc, Mutex};
use anyhow::Result;
use crossterm::{execute, queue, cursor, terminal, style::{self, Stylize, Color}};
use serde_json::json;
use crate::input::{Action, InputManager};
use crate::object::Object;
use crate::tile::Tile;
use crate::game_state::GameState;
use dagr_lib::ems;
use dagr_lib::db::connection;
use dagr_lib::components::world::hex::HexData;
use dagr_lib::core::registry::{EntityKind, FactoryRegistry};
use dagr_lib::bootstrap::{build_factor_registry, AppConfig};
use hecs::{World, Entity};

pub struct Renderer{
  width: u16,
  height: u16,
}

impl Renderer{
  pub fn new(width: u16, height: u16) -> Self{
    Self{width, height,}
  }

  pub fn render(&self, stdout: &mut std::io::Stdout, game_state: &GameState) -> Result<()>{
    let mut buffer = String::new();
    for y in 0..self.height{
      for x in 0..self.width{
        if(y == 0 || y == self.height-1) || (x ==0 || x == self.width-1){
          buffer.push_str(&format!("{}{}", cursor::MoveTo(x, y), "#".white()));
        }
      }
    }

    for y in 1..self.height-1{
      for x in 1..self.width-1{
        let world_x = x as i32 + game_state.camera_x;
        let world_y = y as i32 + game_state.camera_y;
        buffer.push_str(&format!("{}", cursor::MoveTo(x, y)));

        if let Some(hex_entity) = game_state.map.get((world_x, world_y)){
          if let Ok(tile) = game_state.entity_manager.with::<Tile, _, _>(
            hex_entity,
            |tile| tile.clone()
          ){
            buffer.push_str(&format!("{}{}", style::SetForegroundColor(tile.color), tile.symbol));
          }else{
            buffer.push_str(&format!("{}", "?".white()));
          }
        }else{
          buffer.push_str(&format!("{}", " ".white()));
        }
      }
    }
    
    write!(stdout, "{}", buffer)?;
    stdout.flush()?;
    Ok(())
  }

  fn render_hex(&self,
    stdout: &mut std::io::Stdout,
    entity: Entity,
    world: &World
  ) -> Result<()>{
    if let Ok(tile) = world.get::<&Tile>(entity){
      execute!(stdout, style::SetForegroundColor(tile.color))?;
      write!(stdout, "{}", tile.symbol)?;
    }

    Ok(())
  }
}
