use std::io::{self};
use std::sync::Arc;

use anyhow::Result;
use crossterm::{cursor, execute, terminal};
use dagr_lib::{Engine, agency::AgentRuntime};
use tracing::{error, info};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), terminal::LeaveAlternateScreen, cursor::Show);
        let _ = terminal::disable_raw_mode();
    }
}
mod areas;
mod camera;
mod catalog;
mod combat;
mod debug_console;
mod debug_scenario;
mod debug_tui;
mod errors;
mod game_state;
mod gateway;
mod generators;
mod input;
mod inspection;
mod menu;
mod navigation;
mod pathfinding;
mod renderer;
mod scene_playground;
mod startup;
mod tag_playtest;
mod ui;
mod views;
mod visiblity;
mod wilderness_layout;
mod world_map;
use input::{Action, InputManager, InputMode};

#[tokio::main]
async fn main() {
    // Load local development configuration before initializing services. Values
    // already present in the process environment take precedence over `.env`.
    let _ = dotenvy::dotenv();

    let _guard = match init_tracing() {
        Ok(guard) => guard,
        Err(e) => {
            eprintln!("Failed to initialize tracing: {:#}", e);
            std::process::exit(1);
        }
    };

    if let Err(e) = run().await {
        error!("Fatal error: {:#}", e);
        eprintln!("Fatal error: {:#}", e);
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let wilderness_layout = wilderness_layout::WildernessLayout::from_env()?;
    let Some(world) = startup::select().await? else {
        return Ok(());
    };
    let startup::OpenedWorld {
        engine,
        active_player,
        display_name: world_name,
        model_runtimes,
    } = world;
    let agent_runtime = model_runtimes.agent;
    let _structured_output = model_runtimes.structured;

    loop {
        match menu::show_main_menu(&world_name)? {
            menu::MainMenuChoice::Play => {
                run_game(engine.clone(), active_player, wilderness_layout, false).await?
            }
            menu::MainMenuChoice::CombatArena => {
                run_game(engine.clone(), active_player, wilderness_layout, true).await?
            }
            menu::MainMenuChoice::DebugTools => {
                run_debug_tools(engine.clone(), agent_runtime.clone()).await?
            }
            menu::MainMenuChoice::Quit => break,
        }
    }
    Ok(())
}

async fn run_debug_tools(engine: Arc<Engine>, agent_runtime: Arc<AgentRuntime>) -> Result<()> {
    loop {
        match debug_tui::run(engine.clone()).await? {
            debug_tui::DebugDestination::MainMenu => return Ok(()),
            debug_tui::DebugDestination::ScenePlayground => {
                scene_playground::run(engine.clone(), agent_runtime.clone()).await?
            }
            debug_tui::DebugDestination::TagPlaytest => {
                tag_playtest::run(engine.clone(), startup::core_content_path()).await?
            }
        }
    }
}

async fn run_game(
    engine: Arc<Engine>,
    active_player: dagr_lib::characters::CharacterId,
    wilderness_layout: wilderness_layout::WildernessLayout,
    start_in_combat_arena: bool,
) -> Result<()> {
    let mut stdout = io::stdout();
    terminal::enable_raw_mode()?;
    execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide)?;
    let _terminal_guard = TerminalGuard;

    let (w, h) = terminal::size()?;
    let map_height = h / 2;
    let view_w = w.saturating_sub(2);
    let view_h = map_height.saturating_sub(2);

    let mut game_state =
        game_state::GameState::new(engine, active_player, view_w, view_h, wilderness_layout)
            .await?;

    game_state.ensure_starting_hex().await?;
    if start_in_combat_arena {
        game_state.enter_combat_arena().await?;
    }
    game_state
        .camera
        .center_on(game_state.player_x, game_state.player_y);

    let renderer = renderer::Renderer::new(w, h);
    let input = InputManager::new();

    let mut needs_refresh = true;

    loop {
        if game_state.is_auto_navigating() && game_state.popup_message.is_none() {
            game_state.step_navigation().await?;
            needs_refresh = true;
        }

        if needs_refresh {
            renderer.render(&mut stdout, &game_state)?;
            needs_refresh = false;
        }
        std::thread::sleep(std::time::Duration::from_millis(16));

        let input_mode = if game_state.debug.console_open() || game_state.combat.picker_is_open() {
            InputMode::Text
        } else if game_state.debug.editor_open() {
            InputMode::HexEditor
        } else {
            InputMode::Normal
        };
        let action = input.poll_input(input_mode);

        if game_state.debug.console_open() {
            match action {
                Action::ModalInput(ch) => game_state.debug.input(ch),
                Action::ModalBackspace => game_state.debug.backspace(),
                Action::ModalConfirm => game_state.debug.execute(),
                Action::ModalCancel => game_state.debug.close_console(),
                _ => {}
            }
            needs_refresh = true;
            continue;
        }

        if game_state.debug.editor_open() {
            match action {
                Action::Move(dx, dy) => {
                    if let Some(editor) = game_state.debug.editor_mut() {
                        editor.move_cursor(dx, dy);
                        game_state
                            .camera
                            .pan_when_border_reached(editor.cursor.0, editor.cursor.1);
                    }
                }
                Action::EditorField(delta) => {
                    if let Some(editor) = game_state.debug.editor_mut() {
                        editor.select_field(delta)
                    }
                }
                Action::EditorCycle(delta) => {
                    if let Some(editor) = game_state.debug.editor_mut() {
                        editor.cycle(delta)
                    }
                }
                Action::PlaceHex => {
                    let (cursor, profile) = {
                        let editor = game_state.debug.editor().expect("editor is open");
                        (editor.cursor, editor.profile())
                    };
                    let placed = game_state
                        .place_debug_hex(cursor.0, cursor.1, profile)
                        .await?;
                    game_state.debug.set_message(if placed {
                        "Hex created"
                    } else {
                        "Coordinate already occupied"
                    });
                }
                Action::Cancel => game_state.debug.close_editor(),
                _ => {}
            };
            needs_refresh = true;
            continue;
        }
        if game_state.is_looking() {
            match action {
                Action::Move(dx, dy) => {
                    let previous = game_state
                        .look_cursor()
                        .unwrap_or((game_state.player_x, game_state.player_y));
                    if game_state.move_look_cursor(dx, dy) {
                        needs_refresh = true;
                    } else {
                        renderer.render_look_delta(&mut stdout, &game_state, previous)?;
                        needs_refresh = false;
                    }
                }
                Action::Look | Action::Cancel => {
                    game_state.close_look();
                    needs_refresh = true;
                }
                Action::Quit => break,
                _ => {}
            }
            continue;
        }

        match action {
            Action::Quit => break,
            Action::Wait => {
                game_state.wait_turn().await?;
                needs_refresh = true;
            }
            Action::Dismiss => {
                if game_state.popup_message.is_some() {
                    game_state.dismiss_popup();
                    needs_refresh = true;
                }
            }
            Action::Move(dx, dy) => {
                game_state.cancel_navigation();
                game_state.move_player(dx, dy).await?;
                game_state.update_visibility();
                needs_refresh = true;
            }
            Action::Ascend => {
                game_state.cancel_navigation();
                game_state.ascend().await?;
                game_state.update_visibility();
                needs_refresh = true;
            }
            Action::Descend => {
                game_state.cancel_navigation();
                game_state.descend().await?;
                game_state.update_visibility();
                needs_refresh = true;
            }
            Action::Look => {
                if game_state.is_in_world() && game_state.debug.enabled() {
                    game_state
                        .debug
                        .open_editor((game_state.player_x, game_state.player_y));
                } else {
                    game_state.toggle_look();
                }
                needs_refresh = true;
            }
            Action::OpenConsole => {
                game_state.debug.open_console();
                needs_refresh = true;
            }
            Action::Cancel => {
                game_state.dismiss_popup();
                needs_refresh = true;
            }
            Action::Explore => {
                game_state.dismiss_popup();
                game_state.start_exploring();
                needs_refresh = true;
            }
            Action::GenerateDungeon => {
                game_state.generate_dungeon().await?;
                game_state.update_visibility();
                needs_refresh = true;
            }
            Action::OpenMonsterPicker => {
                game_state.open_monster_picker().await?;
                needs_refresh = true;
            }
            Action::ResetArena => {
                game_state.reset_combat_arena().await?;
                needs_refresh = true;
            }
            Action::ModalConfirm => {
                game_state.spawn_selected_monster().await?;
                needs_refresh = true;
            }
            Action::ModalCancel => {
                game_state.combat.close_picker();
                needs_refresh = true;
            }
            Action::ModalBackspace => {
                game_state.combat.picker_backspace();
                needs_refresh = true;
            }
            Action::ModalMove(delta) => {
                game_state.combat.picker_move(delta);
                needs_refresh = true;
            }
            Action::ModalInput(character) => {
                game_state.combat.picker_input(character);
                needs_refresh = true;
            }
            _ => {}
        }
    }

    Ok(())
}

fn init_tracing() -> Result<WorkerGuard> {
    let file_appender = RollingFileAppender::new(Rotation::DAILY, "logs", "dagr_game");

    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_target(true)
                .with_line_number(true),
        )
        .with(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("dagr_game=debug".parse().unwrap())
                .add_directive("trace".parse().unwrap()),
        )
        .init();

    info!("logging initialized");

    Ok(guard)
}
