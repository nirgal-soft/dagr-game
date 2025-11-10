use std::io::{Write, self};
use std::sync::{Arc, Mutex};
use anyhow::Result;
use crossterm::{execute, cursor, terminal};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tracing::{debug, error, info};
mod camera;
mod game_state;
mod input;
mod region_gen;
mod renderer;
mod ui;
mod tile;
mod wilderness_generator;
mod world_map;
use input::{Action, InputManager};

use dagr_lib::ems;
use dagr_lib::db::connection;
use dagr_lib::bootstrap::{build_factor_registry, AppConfig};
use hecs::World;

#[tokio::main]
async fn main(){
  let _guard = match init_tracing(){
    Ok(guard) => guard,
    Err(e) => {
      eprintln!("Failed to initialize tracing: {:#}", e);
      std::process::exit(1);
    }
  };
 
  if let Err(e) = run().await{
    error!("Fatal error: {:#}", e);
    eprintln!("Fatal error: {:#}", e);
    std::process::exit(1);
  }
}

async fn run() -> Result<()>{
  let pool = Arc::new(connection::establish_connection().await?);
  let world = Arc::new(Mutex::new(World::new()));
  let registry = Arc::new(build_factor_registry(AppConfig{
    pool: pool.clone(),
    world_seed: 0
  })?
  );

  let entity_manager = ems::entity_manager::EntityManager::new(
    pool.clone(),
    world.clone(),
    registry
  );

  // let mut rg = region_gen::RegionGenerator::new(entity_manager.clone());
  // rg.generate().await?;
  let _hexes = ems::load::load(&pool, entity_manager.world.clone()).await?;

  let mut stdout = io::stdout();
  terminal::enable_raw_mode()?;
  execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide)?;

  let (w, h) = terminal::size()?;
  let map_height = h / 2;
  let view_w = w.saturating_sub(2);
  let view_h = map_height.saturating_sub(2);

  let mut game_state = game_state::GameState::new(entity_manager, view_w, view_h);

  game_state.player_x = 0;
  game_state.player_y = 0;
  game_state.camera.center_on(game_state.player_x, game_state.player_y);

  let renderer = renderer::Renderer::new(w, h);
  let input = InputManager::new();

  loop{
    std::thread::sleep(std::time::Duration::from_millis(16));
    renderer.render(&mut stdout, &game_state)?;

    match input.poll_input(){
      Action::Quit => break,
      Action::Wait => std::thread::sleep(std::time::Duration::from_millis(16)),
      Action::Move(dx, dy) => {
        game_state.move_player(dx, dy).await?;
      },
      Action::EnterWilderness => {
        info!("entering wilderness");
        game_state.enter_wilderness().await?;
      },
      Action::ExitWilderness => {
        game_state.exit_wilderness()?;
      },
      _ => {},
    }
  }
  execute!(stdout, terminal::LeaveAlternateScreen, cursor::Show)?;
  terminal::disable_raw_mode()?;
  stdout.flush()?;

  Ok(())
}

fn init_tracing() -> Result<WorkerGuard> {
  let file_appender = RollingFileAppender::new(
    Rotation::DAILY,
    "logs",
    "dagr_game"
  );

  let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

  tracing_subscriber::registry()
    .with(
      tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(true)
        .with_line_number(true)
    )
    .with(tracing_subscriber::EnvFilter::from_default_env()
      .add_directive("dagr_game=debug".parse().unwrap())
      .add_directive("trace".parse().unwrap())
    )
    .init();

  info!("logging initialized");

  Ok(guard)
}
