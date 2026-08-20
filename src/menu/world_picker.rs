use anyhow::Result;
use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
};

use crate::catalog::{WorldRecord, display_name_key};

use super::terminal::{TerminalSession, centered, next, previous};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorldSelection {
    Existing(String),
    Create { display_name: String, seed: u64 },
    Quit,
}

pub fn show_world_picker(
    worlds: &[(&str, &WorldRecord)],
    reserved_names: &[&str],
    default_seed: u64,
) -> Result<WorldSelection> {
    let mut terminal = TerminalSession::open()?;
    let mut selected = 0_usize;
    let mut creating = worlds.is_empty();
    let mut editing_seed = false;
    let mut name = String::new();
    let mut seed = default_seed.to_string();
    let mut form_error = None;
    loop {
        terminal.draw(|frame| {
            draw_world_picker(
                frame,
                worlds,
                selected,
                creating,
                editing_seed,
                &name,
                &seed,
                form_error.as_deref(),
            )
        })?;
        let Some(key) = terminal.read_key()? else {
            continue;
        };
        if creating {
            match key {
                KeyCode::Esc if worlds.is_empty() => return Ok(WorldSelection::Quit),
                KeyCode::Esc => {
                    creating = false;
                    editing_seed = false;
                    name.clear();
                    form_error = None;
                }
                KeyCode::Tab | KeyCode::Up | KeyCode::Down => {
                    editing_seed = !editing_seed;
                    form_error = None;
                }
                KeyCode::Backspace => {
                    if editing_seed {
                        seed.pop();
                    } else {
                        name.pop();
                    }
                    form_error = None;
                }
                KeyCode::Enter if !editing_seed && !name.trim().is_empty() => {
                    editing_seed = true;
                    form_error = None;
                }
                KeyCode::Enter if editing_seed => {
                    let display_name = name.trim();
                    if display_name.is_empty() {
                        editing_seed = false;
                        form_error = Some("World name must not be blank".to_string());
                    } else if is_reserved(reserved_names, display_name) {
                        editing_seed = false;
                        form_error = Some("World name is already in use".to_string());
                    } else {
                        match seed.parse::<u64>() {
                            Ok(seed) => {
                                return Ok(WorldSelection::Create {
                                    display_name: display_name.to_string(),
                                    seed,
                                });
                            }
                            Err(_) => {
                                form_error =
                                    Some("Seed must be an unsigned 64-bit integer".to_string())
                            }
                        }
                    }
                }
                KeyCode::Char(character) if !character.is_control() => {
                    if editing_seed {
                        if character.is_ascii_digit() && seed.len() < 20 {
                            seed.push(character);
                        }
                    } else if name.chars().count() < 48 {
                        name.push(character);
                    }
                    form_error = None;
                }
                _ => {}
            }
            continue;
        }
        let item_count = worlds.len() + 2;
        match key {
            KeyCode::Up | KeyCode::Char('k') => selected = previous(selected, item_count),
            KeyCode::Down | KeyCode::Char('j') => selected = next(selected, item_count),
            KeyCode::Home => selected = 0,
            KeyCode::End => selected = item_count - 1,
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                return Ok(WorldSelection::Quit);
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                creating = true;
                editing_seed = false;
                name.clear();
            }
            KeyCode::Enter if selected < worlds.len() => {
                return Ok(WorldSelection::Existing(worlds[selected].0.to_string()));
            }
            KeyCode::Enter if selected == worlds.len() => {
                creating = true;
                editing_seed = false;
                name.clear();
            }
            KeyCode::Enter => return Ok(WorldSelection::Quit),
            _ => {}
        }
    }
}

fn is_reserved(reserved_names: &[&str], candidate: &str) -> bool {
    let candidate = display_name_key(candidate);
    reserved_names
        .iter()
        .any(|reserved| display_name_key(reserved) == candidate)
}

#[allow(clippy::too_many_arguments)]
fn draw_world_picker(
    frame: &mut Frame<'_>,
    worlds: &[(&str, &WorldRecord)],
    selected: usize,
    creating: bool,
    editing_seed: bool,
    name: &str,
    seed: &str,
    form_error: Option<&str>,
) {
    let panel = centered(frame.area(), 72, 72);
    frame.render_widget(Clear, panel);
    if creating {
        let prompt = if worlds.is_empty() {
            "No worlds yet. Create the first world:"
        } else {
            "Create a new world:"
        };
        let inactive = Style::default().fg(Color::Gray);
        let active = Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD);
        frame.render_widget(
            Paragraph::new(vec![
                Line::styled(prompt, Style::default().fg(Color::LightCyan)),
                Line::raw(""),
                Line::styled(
                    format!("World name  {name}{}", if editing_seed { "" } else { "▌" }),
                    if editing_seed { inactive } else { active },
                ),
                Line::styled(
                    format!("World seed  {seed}{}", if editing_seed { "▌" } else { "" }),
                    if editing_seed { active } else { inactive },
                ),
                Line::raw(""),
                Line::styled(
                    form_error.unwrap_or("Tab switch field   Enter continue/create   Esc cancel"),
                    if form_error.is_some() {
                        Style::default().fg(Color::LightRed)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    },
                ),
            ])
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .title(" CREATE WORLD ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::LightCyan)),
            ),
            panel,
        );
        return;
    }
    let mut items = worlds
        .iter()
        .map(|(_, world)| {
            ListItem::new(vec![
                Line::styled(
                    &world.display_name,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Line::styled("  Ready", Style::default().fg(Color::DarkGray)),
            ])
        })
        .collect::<Vec<_>>();
    items.push(ListItem::new(Line::styled(
        "Create new world",
        Style::default().fg(Color::LightCyan),
    )));
    items.push(ListItem::new(Line::styled(
        "Quit",
        Style::default().fg(Color::DarkGray),
    )));
    let mut state = ListState::default().with_selected(Some(selected));
    frame.render_stateful_widget(
        List::new(items)
            .block(
                Block::default()
                    .title(" SELECT A WORLD ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Rgb(35, 90, 105))),
            )
            .highlight_symbol("  › ")
            .highlight_style(Style::default().bg(Color::Rgb(15, 35, 42))),
        panel,
        &mut state,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserved_names_are_compared_case_insensitively() {
        assert!(is_reserved(&["Grey March"], "grey march"));
        assert!(!is_reserved(&["Grey March"], "The Long Road"));
    }
}
