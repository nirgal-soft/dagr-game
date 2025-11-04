use std::io::{Write, self};
use std::sync::{Arc, Mutex};
use anyhow::Result;
use crossterm::{execute, queue, cursor, terminal, style::{self, Stylize, Color}};
use serde_json::json;
mod input;
mod map;
mod object;
mod tile;
mod region_gen;
mod renderer;
mod game_state;
use input::{Action, InputManager};
use object::Object;
use tile::Tile;

use dagr_lib::ems;
use dagr_lib::db::connection;
use dagr_lib::components::world::hex::Hex;
use dagr_lib::core::registry::{EntityKind, FactoryRegistry};
use dagr_lib::bootstrap::{build_factor_registry, AppConfig};
use hecs::{World, Entity};

#[tokio::main]
async fn main() -> Result<()> {
  let pool = Arc::new(connection::establish_connection().await?);
  let mut world = Arc::new(Mutex::new(World::new()));
  let registry = Arc::new(build_factor_registry(AppConfig{pool: pool.clone(), world_seed: 0})?);
  let entity_manager = ems::entity_manager::EntityManager::new(pool.clone(), world.clone(), registry);
  let rg = region_gen::RegionGenerator::new(entity_manager.clone());
  let hexes = ems::load::load(&pool, world.clone()).await?;

  let mut stdout = io::stdout();
  terminal::enable_raw_mode()?;
  execute!(stdout, terminal::EnterAlternateScreen)?;
  execute!(stdout, terminal::Clear(terminal::ClearType::All))?;
  execute!(stdout, cursor::Hide)?;
  let input = InputManager::new();
  let mut objects: Vec<Object> = vec![];
  objects.push(Object::new(Tile::new('@', Color::Green), 10, 10));

  loop{
    std::thread::sleep(std::time::Duration::from_millis(16));
    render(&stdout, &objects)?;
    let action = input.poll_input();
    match action{
      Action::Quit => break,
      _ => {},
    }
  }
  execute!(stdout, terminal::LeaveAlternateScreen)?;
  execute!(stdout, cursor::Show)?;
  terminal::disable_raw_mode()?;
  stdout.flush()?;

  for hex in hexes{
    println!("{:?}: ", hex);
    let hd = entity_manager.get_component::<Hex, _>(hex);
    println!("{:?}", hd);
  }

  Ok(())
}

fn render(mut stdout: &io::Stdout, objects: &Vec<Object>) -> io::Result<()>{
  let mut buffer = String::new();
  buffer.push_str(&format!("{}", cursor::MoveTo(0, 0)));

  for y in 0..40{
    for x in 0..150{
      if(y == 0 || y == 40-1) || (x == 0 || x == 150-1){
        buffer.push_str(&format!("{}{}", cursor::MoveTo(x, y), "#".white()));
      }
    }
  }

  for object in objects{
    buffer.push_str(&format!(
        "{}{}{}",
        cursor::MoveTo(object.x, object.y),
        style::SetForegroundColor(object.tile.color),
        object.tile.symbol
      ));
  }
  write!(stdout, "{}", buffer)?;
  stdout.flush()?;
  Ok(())
}
