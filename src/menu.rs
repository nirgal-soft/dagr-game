use std::{io, time::Duration};

use anyhow::Result;
use crossterm::{
  cursor,
  event::{self, Event, KeyCode, KeyEventKind},
  execute,
  terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
  Frame, Terminal,
  backend::CrosstermBackend,
  layout::{Alignment, Constraint, Direction, Layout, Rect},
  style::{Color, Modifier, Style},
  text::{Line, Span, Text},
  widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MainMenuChoice {
  Play,
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

const ENTRIES: [MenuEntry; 3] = [
  MenuEntry {
    choice: MainMenuChoice::Play,
    key: "P",
    title: "ENTER THE WORLD",
    subtitle: "Continue into the roguelike reference client",
    detail: "Explore the persistent world, move between areas, and exercise the playable client as it grows.",
  },
  MenuEntry {
    choice: MainMenuChoice::DebugTools,
    key: "D",
    title: "ENGINE WORKBENCH",
    subtitle: "Shape and inspect the campaign",
    detail: "Browse actors, invoke scoped tools, frame scenes, create threats, and inspect canonical consequences.",
  },
  MenuEntry {
    choice: MainMenuChoice::Quit,
    key: "Q",
    title: "LEAVE FOR NOW",
    subtitle: "Return to the road another day",
    detail: "Close DAGR cleanly. The persistent world and campaign remain exactly as you left them.",
  },
];

struct MenuGuard;
impl Drop for MenuGuard {
  fn drop(&mut self) {
    let _ = terminal::disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen, cursor::Show);
  }
}

pub fn show_main_menu(world_seed: u64) -> Result<MainMenuChoice> {
  terminal::enable_raw_mode()?;
  execute!(io::stdout(), EnterAlternateScreen, cursor::Hide)?;
  let _guard = MenuGuard;
  let backend = CrosstermBackend::new(io::stdout());
  let mut terminal = Terminal::new(backend)?;
  terminal.clear()?;
  let mut selected = 0usize;
  let mut help = false;

  loop {
    terminal.draw(|frame| draw(frame, selected, help, world_seed))?;
    if !event::poll(Duration::from_millis(100))? {
      continue;
    }
    let Event::Key(key) = event::read()? else {
      continue;
    };
    if key.kind != KeyEventKind::Press {
      continue;
    }
    if help {
      help = false;
      continue;
    }
    match key.code {
      KeyCode::Up | KeyCode::Char('k') => selected = previous(selected, ENTRIES.len()),
      KeyCode::Down | KeyCode::Char('j') => selected = next(selected, ENTRIES.len()),
      KeyCode::Home => selected = 0,
      KeyCode::End => selected = ENTRIES.len() - 1,
      KeyCode::Enter => return Ok(ENTRIES[selected].choice),
      KeyCode::Char('1') | KeyCode::Char('p') | KeyCode::Char('P') => {
        return Ok(MainMenuChoice::Play);
      }
      KeyCode::Char('2') | KeyCode::Char('d') | KeyCode::Char('D') => {
        return Ok(MainMenuChoice::DebugTools);
      }
      KeyCode::Char('3') | KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
        return Ok(MainMenuChoice::Quit);
      }
      KeyCode::Char('?') => help = true,
      _ => {}
    }
  }
}

fn next(selected: usize, len: usize) -> usize {
  if len == 0 { 0 } else { (selected + 1) % len }
}
fn previous(selected: usize, len: usize) -> usize {
  if len == 0 {
    0
  } else if selected == 0 {
    len - 1
  } else {
    selected - 1
  }
}

fn draw(frame: &mut Frame<'_>, selected: usize, help: bool, world_seed: u64) {
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
  let seed = Line::from(vec![
    Span::styled(
      " WORLD SEED ",
      Style::default()
        .fg(Color::Black)
        .bg(Color::Rgb(120, 105, 45))
        .add_modifier(Modifier::BOLD),
    ),
    Span::styled(
      format!(" {world_seed} "),
      Style::default().fg(Color::LightYellow),
    ),
    Span::raw("   "),
    Span::styled(
      " PostgreSQL connected • canonical state ready ",
      Style::default().fg(Color::DarkGray),
    ),
  ]);
  frame.render_widget(Paragraph::new(seed).alignment(Alignment::Center), rows[4]);

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
  let text = "WELCOME TO DAGR\n\nThe playable client and engine workbench share one persistent PostgreSQL world.\n\n↑ ↓ or j k     move selection\nEnter          open selection\nP              enter the world\nD              open engine workbench\nQ / Esc        quit\n?              show this help\n\nBoth clients restore the terminal before returning here.\n\nPress any key to close help.";
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

fn centered(area: Rect, width_percent: u16, height_percent: u16) -> Rect {
  let width = area
    .width
    .saturating_mul(width_percent)
    .saturating_div(100)
    .max(20)
    .min(area.width);
  let height = area
    .height
    .saturating_mul(height_percent)
    .saturating_div(100)
    .max(10)
    .min(area.height);
  Rect {
    x: area.x + (area.width - width) / 2,
    y: area.y + (area.height - height) / 2,
    width,
    height,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn menu_navigation_wraps() {
    assert_eq!(next(2, 3), 0);
    assert_eq!(previous(0, 3), 2);
  }
  #[test]
  fn menu_choices_are_distinct() {
    assert_ne!(MainMenuChoice::Play, MainMenuChoice::DebugTools);
    assert_ne!(MainMenuChoice::DebugTools, MainMenuChoice::Quit);
  }
}
