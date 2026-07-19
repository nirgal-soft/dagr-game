use std::io::{self};
use std::sync::{Arc, Mutex};
use anyhow::Result;
use crossterm::{execute, cursor, terminal};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tracing::{error, info};

struct TerminalGuard;

impl Drop for TerminalGuard {
  fn drop(&mut self) {
    let _ = execute!(io::stdout(), terminal::LeaveAlternateScreen, cursor::Show);
    let _ = terminal::disable_raw_mode();
  }
}
mod areas;
mod camera;
mod combat;
mod debug_console;
mod debug_scenario;
mod debug_tui;
mod errors;
mod game_state;
mod generators;
mod input;
mod inspection;
mod menu;
mod scene_playground;
mod navigation;
mod pathfinding;
mod region_gen;
mod renderer;
mod seed;
mod ui;
mod views;
mod visiblity;
mod wilderness_layout;
mod world_map;
use input::{Action, InputManager, InputMode};

use dagr_lib::ems;
use dagr_lib::db::connection;
use dagr_lib::bootstrap::{build_factor_registry, AppConfig};
use hecs::World;

#[tokio::main]
async fn main(){
  // Load local development configuration before initializing services. Values
  // already present in the process environment take precedence over `.env`.
  let _ = dotenvy::dotenv();

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
  let world_seed = std::env::var("WORLD_SEED")
    .unwrap_or_else(|_| "0".to_string())
    .parse::<u64>()?;
  let pool = Arc::new(connection::establish_connection().await?);
  let wilderness_layout = wilderness_layout::WildernessLayout::from_env()?;

  loop{
    match menu::show_main_menu(world_seed)?{
      menu::MainMenuChoice::Play => run_game(pool.clone(), world_seed, wilderness_layout, false).await?,
      menu::MainMenuChoice::CombatArena => run_game(pool.clone(), world_seed, wilderness_layout, true).await?,
      menu::MainMenuChoice::DebugTools => run_debug_tools(pool.clone()).await?,
      menu::MainMenuChoice::Quit => break,
    }
  }
  Ok(())
}

async fn run_debug_tools(pool: Arc<sqlx::PgPool>) -> Result<()>{
  loop{
    match debug_tui::run(pool.clone()).await?{
      debug_tui::DebugDestination::MainMenu => return Ok(()),
      debug_tui::DebugDestination::ScenePlayground => scene_playground::run(pool.clone()).await?,
    }
  }
}

async fn run_game(
  pool: Arc<sqlx::PgPool>,
  world_seed: u64,
  wilderness_layout: wilderness_layout::WildernessLayout,
  start_in_combat_arena: bool,
) -> Result<()>{
  let world = Arc::new(Mutex::new(World::new()));
  let registry = Arc::new(
    build_factor_registry(AppConfig{
      pool: pool.clone(),
      world_seed
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
  let _terminal_guard = TerminalGuard;

  let (w, h) = terminal::size()?;
  let map_height = h / 2;
  let view_w = w.saturating_sub(2);
  let view_h = map_height.saturating_sub(2);

  let mut game_state = game_state::GameState::new(
    entity_manager,
    pool.clone(),
    view_w,
    view_h,
    wilderness_layout,
  );

  game_state.player_x = 0;
  game_state.player_y = 0;
  game_state.ensure_starting_hex().await?;
  if start_in_combat_arena{
    game_state.enter_combat_arena().await?;
  }
  game_state.camera.center_on(game_state.player_x, game_state.player_y);

  let renderer = renderer::Renderer::new(w, h);
  let input = InputManager::new();

  let mut needs_refresh = true;

  loop{
    if game_state.is_auto_navigating() && game_state.popup_message.is_none(){
      game_state.step_navigation().await?;
      needs_refresh = true;
    }

    if needs_refresh{
      renderer.render(&mut stdout, &game_state)?;
      needs_refresh = false;
    }
    std::thread::sleep(std::time::Duration::from_millis(16));

    let input_mode=if game_state.debug.console_open() || game_state.combat.picker_is_open(){
      InputMode::Text
    }else if game_state.debug.editor_open(){
      InputMode::HexEditor
    }else{
      InputMode::Normal
    };
    let action=input.poll_input(input_mode);

    if game_state.debug.console_open(){
      match action{
        Action::ModalInput(ch)=>game_state.debug.input(ch),
        Action::ModalBackspace=>game_state.debug.backspace(),
        Action::ModalConfirm=>game_state.debug.execute(),
        Action::ModalCancel=>game_state.debug.close_console(),
        _=>{},
      }
      needs_refresh=true;
      continue
    }

    if game_state.debug.editor_open(){
      match action{
        Action::Move(dx,dy)=>{
          if let Some(editor)=game_state.debug.editor_mut(){
            editor.move_cursor(dx,dy);
            game_state.camera.pan_when_border_reached(editor.cursor.0,editor.cursor.1);
          }
        },
        Action::EditorField(delta)=>{if let Some(editor)=game_state.debug.editor_mut(){editor.select_field(delta)}},
        Action::EditorCycle(delta)=>{if let Some(editor)=game_state.debug.editor_mut(){editor.cycle(delta)}},
        Action::PlaceHex=>{
          let (cursor,profile)={
            let editor=game_state.debug.editor().expect("editor is open");
            (editor.cursor,editor.profile())
          };
          let placed=game_state.place_debug_hex(cursor.0,cursor.1,profile).await?;
          game_state.debug.set_message(if placed{"Hex created"}else{"Coordinate already occupied"});
        },
        Action::Cancel=>game_state.debug.close_editor(),
        _=>{},
      };
      needs_refresh=true;
      continue
    }
    if game_state.is_looking(){
      match action{
        Action::Move(dx,dy) => {
          let previous=game_state.look_cursor().unwrap_or((game_state.player_x,game_state.player_y));
          if game_state.move_look_cursor(dx,dy){
            needs_refresh=true;
          }else{
            renderer.render_look_delta(&mut stdout,&game_state,previous)?;
            needs_refresh=false;
          }
        },
        Action::Look | Action::Cancel => {
          game_state.close_look();
          needs_refresh=true;
        },
        Action::Quit => break,
        _ => {},
      }
      continue
    }

    match action{
      Action::Quit => break,
      Action::Wait => {
        game_state.wait_turn().await?;
        needs_refresh = true;
      },
      Action::Dismiss => {
        if game_state.popup_message.is_some(){
          game_state.dismiss_popup();
          needs_refresh = true;
        }
      }
      Action::Move(dx, dy) => {
        game_state.cancel_navigation();
        game_state.move_player(dx, dy).await?;
        game_state.update_visibility();
        needs_refresh = true;
      },
      Action::Ascend => {
        game_state.cancel_navigation();
        game_state.ascend().await?;
        game_state.update_visibility();
        needs_refresh = true;
      },
      Action::Descend => {
        game_state.cancel_navigation();
        game_state.descend().await?;
        game_state.update_visibility();
        needs_refresh = true;
      },
      Action::Look => {
        if game_state.is_in_world() && game_state.debug.enabled(){
          game_state.debug.open_editor((game_state.player_x,game_state.player_y));
        }else{
          game_state.toggle_look();
        }
        needs_refresh = true;
      },
      Action::OpenConsole => {
        game_state.debug.open_console();
        needs_refresh = true;
      },
      Action::Cancel => {
        game_state.dismiss_popup();
        needs_refresh = true;
      },
      Action::Explore => {
        game_state.dismiss_popup();
        game_state.start_exploring();
        needs_refresh = true;
      },
      Action::GenerateDungeon => {
        game_state.generate_dungeon().await?;
        game_state.update_visibility();
        needs_refresh = true;
      },
      Action::OpenMonsterPicker => {
        game_state.open_monster_picker().await?;
        needs_refresh = true;
      },
      Action::ResetArena => {
        game_state.reset_combat_arena().await?;
        needs_refresh = true;
      },
      Action::ModalConfirm => {
        game_state.spawn_selected_monster().await?;
        needs_refresh = true;
      },
      Action::ModalCancel => {
        game_state.combat.close_picker();
        needs_refresh = true;
      },
      Action::ModalBackspace => {
        game_state.combat.picker_backspace();
        needs_refresh = true;
      },
      Action::ModalMove(delta) => {
        game_state.combat.picker_move(delta);
        needs_refresh = true;
      },
      Action::ModalInput(character) => {
        game_state.combat.picker_input(character);
        needs_refresh = true;
      },
      _ => {},
    }
  }

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
