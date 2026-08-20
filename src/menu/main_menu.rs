use anyhow::Result;
use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};

use super::terminal::{TerminalSession, centered, next, previous};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MainMenuChoice {
    Play,
    CombatArena,
    DebugTools,
    Quit,
}

#[derive(Clone, Copy)]
struct MenuEntry {
    choice: MainMenuChoice,
    key: &'static str,
    title: &'static str,
    subtitle: &'static str,
    detail: &'static str,
}

const ENTRIES: [MenuEntry; 4] = [
    MenuEntry {
        choice: MainMenuChoice::Play,
        key: "P",
        title: "ENTER THE WORLD",
        subtitle: "Continue into the roguelike reference client",
        detail: "Explore the persistent world, move between areas, and exercise the playable client as it grows.",
    },
    MenuEntry {
        choice: MainMenuChoice::CombatArena,
        key: "A",
        title: "COMBAT ARENA",
        subtitle: "Enter the persistent stone-circle test ground",
        detail: "Spawn combatants, exercise simple procedures, and build the combat loop without disturbing the ordinary world.",
    },
    MenuEntry {
        choice: MainMenuChoice::DebugTools,
        key: "D",
        title: "ENGINE WORKBENCH",
        subtitle: "Shape and inspect the campaign",
        detail: "Browse actors, invoke scoped tools, inspect canonical consequences, or play a tiny AI-powered scene.",
    },
    MenuEntry {
        choice: MainMenuChoice::Quit,
        key: "Q",
        title: "LEAVE FOR NOW",
        subtitle: "Return to the road another day",
        detail: "Close DAGR cleanly. The persistent world and campaign remain exactly as you left them.",
    },
];

pub fn show_main_menu(world_name: &str) -> Result<MainMenuChoice> {
    let mut terminal = TerminalSession::open()?;
    let mut selected = 0_usize;
    let mut help = false;

    loop {
        terminal.draw(|frame| draw(frame, selected, help, world_name))?;
        let Some(key) = terminal.read_key()? else {
            continue;
        };
        if help {
            help = false;
            continue;
        }
        match key {
            KeyCode::Up | KeyCode::Char('k') => selected = previous(selected, ENTRIES.len()),
            KeyCode::Down | KeyCode::Char('j') => selected = next(selected, ENTRIES.len()),
            KeyCode::Home => selected = 0,
            KeyCode::End => selected = ENTRIES.len() - 1,
            KeyCode::Enter => return Ok(ENTRIES[selected].choice),
            KeyCode::Char('1') | KeyCode::Char('p') | KeyCode::Char('P') => {
                return Ok(MainMenuChoice::Play);
            }
            KeyCode::Char('2') | KeyCode::Char('a') | KeyCode::Char('A') => {
                return Ok(MainMenuChoice::CombatArena);
            }
            KeyCode::Char('3') | KeyCode::Char('d') | KeyCode::Char('D') => {
                return Ok(MainMenuChoice::DebugTools);
            }
            KeyCode::Char('4') | KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                return Ok(MainMenuChoice::Quit);
            }
            KeyCode::Char('?') => help = true,
            _ => {}
        }
    }
}

fn draw(frame: &mut Frame<'_>, selected: usize, help: bool, world_name: &str) {
    let area = frame.area();
    if area.width < 24 || area.height < 12 {
        return;
    }
    let outer = centered(area, 90, 92);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),
            Constraint::Length(2),
            Constraint::Min(12),
            Constraint::Length(5),
            Constraint::Length(3),
        ])
        .split(outer);

    let logo = Text::from(vec![
        Line::styled(
            "██████╗  █████╗  ██████╗ ██████╗",
            Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
        ),
        Line::styled(
            "██╔══██╗██╔══██╗██╔════╝ ██╔══██╗",
            Style::default().fg(Color::Cyan),
        ),
        Line::styled(
            "██║  ██║███████║██║  ███╗██████╔╝",
            Style::default().fg(Color::Cyan),
        ),
        Line::styled(
            "██║  ██║██╔══██║██║   ██║██╔══██╗",
            Style::default().fg(Color::DarkGray),
        ),
        Line::styled(
            "██████╔╝██║  ██║╚██████╔╝██║  ██║",
            Style::default().fg(Color::DarkGray),
        ),
        Line::styled(
            "╚═════╝ ╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═╝",
            Style::default().fg(Color::DarkGray),
        ),
    ]);
    frame.render_widget(Paragraph::new(logo).alignment(Alignment::Center), rows[0]);
    frame.render_widget(
        Paragraph::new("A PERSISTENT WORLD • A CREATIVE GAME MASTER")
            .alignment(Alignment::Center)
            .style(
                Style::default()
                    .fg(Color::LightYellow)
                    .add_modifier(Modifier::BOLD),
            ),
        rows[1],
    );

    let items: Vec<ListItem> = ENTRIES
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let active = index == selected;
            let key_style = if active {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::LightCyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            let title_style = if active {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(format!(" {} ", entry.key), key_style),
                    Span::raw("  "),
                    Span::styled(entry.title, title_style),
                ]),
                Line::styled(
                    format!("      {}", entry.subtitle),
                    if active {
                        Style::default().fg(Color::LightCyan)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    },
                ),
                Line::raw(""),
            ])
        })
        .collect();
    let mut state = ListState::default().with_selected(Some(selected));
    let list = List::new(items)
        .block(
            Block::default()
                .title(" CHOOSE YOUR PATH ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(35, 90, 105))),
        )
        .highlight_style(Style::default().bg(Color::Rgb(15, 35, 42)))
        .highlight_symbol("  › ");
    frame.render_stateful_widget(list, rows[2], &mut state);

    let entry = ENTRIES[selected];
    frame.render_widget(
        Paragraph::new(entry.detail)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .block(
                Block::default()
                    .title(" NOW ")
                    .borders(Borders::TOP)
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .style(Style::default().fg(Color::Gray)),
        rows[3],
    );
    let world = Line::from(vec![
        Span::styled(
            " WORLD ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Rgb(120, 105, 45))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" {world_name} "),
            Style::default().fg(Color::LightYellow),
        ),
        Span::raw("   "),
        Span::styled(
            " SQLite world open • canonical state ready ",
            Style::default().fg(Color::DarkGray),
        ),
    ]);
    frame.render_widget(Paragraph::new(world).alignment(Alignment::Center), rows[4]);

    let footer = Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(2),
        width: area.width,
        height: 2,
    };
    frame.render_widget(
        Paragraph::new("↑↓ / j k navigate   Enter select   P play   D workbench   ? help   Q quit")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray)),
        footer,
    );
    if help {
        draw_help(frame, area)
    }
}

fn draw_help(frame: &mut Frame<'_>, area: Rect) {
    let popup = centered(area, 58, 50);
    frame.render_widget(Clear, popup);
    let text = "WELCOME TO DAGR\n\nThe playable client and engine workbench share one managed SQLite world.\n\n↑ ↓ or j k     move selection\nEnter          open selection\nP              enter the world\nD              open engine workbench\nQ / Esc        quit\n?              show this help\n\nBoth clients restore the terminal before returning here.\n\nPress any key to close help.";
    frame.render_widget(
        Paragraph::new(text)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .block(
                Block::default()
                    .title(" THE ROAD AHEAD ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::LightCyan)),
            ),
        popup,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_choices_are_distinct() {
        assert_ne!(MainMenuChoice::Play, MainMenuChoice::CombatArena);
        assert_ne!(MainMenuChoice::CombatArena, MainMenuChoice::DebugTools);
        assert_ne!(MainMenuChoice::DebugTools, MainMenuChoice::Quit);
    }
}
