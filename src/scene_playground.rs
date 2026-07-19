use std::{io, sync::Arc, time::Duration};

use anyhow::Result;
use crossterm::{
  cursor,
  event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
  execute,
  terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use dagr_lib::{
  agency::{ActorDirectoryEntry, ActorScope, AgentRunner, load_actor_directory},
  llm::claude::{ClaudeProvider, DEFAULT_CLAUDE_MODEL},
};
use ratatui::{
  Frame, Terminal,
  backend::CrosstermBackend,
  layout::{Alignment, Constraint, Direction, Layout, Rect},
  style::{Color, Modifier, Style},
  text::{Line, Span, Text},
  widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};
use serde_json::json;
use sqlx::PgPool;

struct PlaygroundGuard;
impl Drop for PlaygroundGuard {
  fn drop(&mut self) {
    let _ = terminal::disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen, cursor::Show);
  }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Focus {
  Actors,
  Composer,
}
#[derive(Clone, Copy)]
enum MessageRole {
  You,
  Model,
  Tool,
  System,
  Error,
}
struct ChatMessage {
  role: MessageRole,
  content: String,
}

struct Playground {
  actors: Vec<ActorDirectoryEntry>,
  actor_cursor: usize,
  actor_filter: String,
  actor: ActorScope,
  focus: Focus,
  searching: bool,
  input: String,
  messages: Vec<ChatMessage>,
  scroll: u16,
  status: String,
  help: bool,
  model: String,
  show_details: bool,
  starter_index: usize,
}

impl Playground {
  async fn load(pool: &PgPool) -> Result<Self> {
    let actors = load_actor_directory(pool).await?;
    let actor = actors
      .first()
      .map(|entry| entry.actor)
      .unwrap_or(ActorScope::Gm);
    let model =
      std::env::var("DAGR_LLM_MODEL").unwrap_or_else(|_| DEFAULT_CLAUDE_MODEL.to_string());
    let configured = std::env::var("ANTHROPIC_API_KEY").is_ok_and(|key| !key.trim().is_empty());
    Ok(Self{
      actors,actor_cursor:0,actor_filter:String::new(),actor,focus:Focus::Actors,
      searching:false,input:String::new(),scroll:0,help:false,model:model.clone(),show_details:false,starter_index:0,
      status:if configured{"Choose an actor, press F2 for a scene starter, then play.".to_string()}else{"Set ANTHROPIC_API_KEY in .env to play a scene.".to_string()},
      messages:vec![ChatMessage{role:MessageRole::System,content:
        "Play one small moment. Choose who animates the scene, start with F2 or your own premise, and respond naturally for a few turns.".to_string()}],
    })
  }
  fn filtered_indices(&self) -> Vec<usize> {
    let needle = self.actor_filter.to_ascii_lowercase();
    self
      .actors
      .iter()
      .enumerate()
      .filter(|(_, entry)| {
        needle.is_empty()
          || entry.name.to_ascii_lowercase().contains(&needle)
          || entry.summary.to_ascii_lowercase().contains(&needle)
      })
      .map(|(index, _)| index)
      .collect()
  }
  fn move_actor(&mut self, delta: isize) {
    let len = self.filtered_indices().len();
    if len == 0 {
      self.actor_cursor = 0;
      return;
    }
    self.actor_cursor = if delta < 0 {
      self.actor_cursor.saturating_sub(1)
    } else {
      (self.actor_cursor + 1).min(len - 1)
    };
  }
  fn activate_actor(&mut self) {
    if let Some(index) = self.filtered_indices().get(self.actor_cursor).copied() {
      self.actor = self.actors[index].actor;
      self.messages.push(ChatMessage {
        role: MessageRole::System,
        content: format!(
          "Actor scope changed to {} — {}.",
          self.actors[index].name, self.actors[index].summary
        ),
      });
      self.status = format!("{} is ready for a short scene", self.actors[index].name);
      self.focus = Focus::Composer;
    }
  }
  fn load_starter(&mut self) {
    let starters = scene_starters(self.actor);
    self.input = starters[self.starter_index % starters.len()].to_string();
    self.starter_index = (self.starter_index + 1) % starters.len();
    self.status = "Scene starter loaded — edit it or press Enter to begin.".to_string();
  }
  fn new_scene(&mut self) {
    self.input.clear();
    self.scroll = 0;
    self.messages = vec![ChatMessage {
      role: MessageRole::System,
      content: "A fresh moment begins. Press F2 for a premise or describe what is happening."
        .to_string(),
    }];
    self.status = "New scene ready.".to_string();
  }
  fn history(&self) -> Vec<serde_json::Value> {
    self.messages.iter().rev().take(12).rev().map(|message|json!({
      "role":match message.role{MessageRole::You=>"user",MessageRole::Model=>"model",MessageRole::Tool=>"tool",MessageRole::System=>"system",MessageRole::Error=>"error"},
      "content":message.content,
    })).collect()
  }
}

pub async fn run(pool: Arc<PgPool>) -> Result<()> {
  terminal::enable_raw_mode()?;
  execute!(io::stdout(), EnterAlternateScreen, cursor::Hide)?;
  let _guard = PlaygroundGuard;
  let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
  terminal.clear()?;
  let mut app = Playground::load(pool.as_ref()).await?;

  loop {
    terminal.draw(|frame| draw(frame, &app))?;
    if !event::poll(Duration::from_millis(100))? {
      continue;
    }
    let Event::Key(key) = event::read()? else {
      continue;
    };
    if key.kind != KeyEventKind::Press {
      continue;
    }
    if app.help {
      app.help = false;
      continue;
    }
    if app.searching {
      match key.code {
        KeyCode::Esc => {
          app.actor_filter.clear();
          app.searching = false;
        }
        KeyCode::Enter => {
          app.searching = false;
          app.activate_actor();
        }
        KeyCode::Backspace => {
          app.actor_filter.pop();
          app.actor_cursor = 0;
        }
        KeyCode::Up => app.move_actor(-1),
        KeyCode::Down => app.move_actor(1),
        KeyCode::Char(character) => {
          app.actor_filter.push(character);
          app.actor_cursor = 0;
        }
        _ => {}
      }
      continue;
    }
    match key.code {
      KeyCode::F(2) => {
        app.load_starter();
        continue;
      }
      KeyCode::F(3) => {
        app.new_scene();
        continue;
      }
      KeyCode::Char('v') if app.input.is_empty() => {
        app.show_details = !app.show_details;
        continue;
      }
      _ => {}
    }
    match app.focus {
      Focus::Actors => match key.code {
        KeyCode::Char('q') | KeyCode::Esc => break,
        KeyCode::Char('?') => app.help = true,
        KeyCode::Char('/') => app.searching = true,
        KeyCode::Tab => app.focus = Focus::Composer,
        KeyCode::Up | KeyCode::Char('k') => app.move_actor(-1),
        KeyCode::Down | KeyCode::Char('j') => app.move_actor(1),
        KeyCode::Enter => app.activate_actor(),
        _ => {}
      },
      Focus::Composer => match key.code {
        KeyCode::Esc if app.input.is_empty() => break,
        KeyCode::Esc => app.input.clear(),
        KeyCode::Tab => app.focus = Focus::Actors,
        KeyCode::Char('?') if app.input.is_empty() => app.help = true,
        KeyCode::PageUp => app.scroll = app.scroll.saturating_add(8),
        KeyCode::PageDown => app.scroll = app.scroll.saturating_sub(8),
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => app.input.clear(),
        KeyCode::Backspace => {
          app.input.pop();
        }
        KeyCode::Enter if !app.input.trim().is_empty() => {
          send(&mut terminal, &mut app, pool.clone()).await
        }
        KeyCode::Char(character) => app.input.push(character),
        _ => {}
      },
    }
  }
  Ok(())
}

async fn send(
  terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
  app: &mut Playground,
  pool: Arc<PgPool>,
) {
  let message = std::mem::take(&mut app.input);
  app.messages.push(ChatMessage {
    role: MessageRole::You,
    content: message.clone(),
  });
  let history = app.history();
  app.status = format!("{} is thinking…", app.model);
  let _ = terminal.draw(|frame| draw(frame, app));
  let result = async {
    let provider = ClaudeProvider::from_env()?;
    let mut runner = AgentRunner::new(provider, pool, 6)?;
    runner.run_with_stimulus(app.actor,json!({
      "kind":"gameplay_scene_turn","message":message,"conversation":history,
      "instruction":"Create the smallest fun playable unit: one immediate situation, one meaningful pressure, and a clear invitation for the player to act. Respond naturally, keep the scene moving, and use scoped tools whenever canonical state changes. Do not overbuild the campaign."
    })).await
  }
  .await;
  match result {
    Ok(report) => {
      let had_visible_text = !report.visible_text.is_empty();
      for text in report.visible_text {
        app.messages.push(ChatMessage {
          role: MessageRole::Model,
          content: text,
        });
      }
      for tool in &report.tool_results {
        let (role, content) = if tool.success {
          (
            MessageRole::Tool,
            format!(
              "The world changed: {}{}",
              tool
                .event
                .as_ref()
                .map(|event| event.event_type.as_str())
                .unwrap_or(&tool.tool_name),
              if app.show_details {
                tool
                  .event
                  .as_ref()
                  .map(|event| {
                    format!(
                      "\n\nTechnical detail — event #{}\n{}",
                      event.id,
                      serde_json::to_string_pretty(&event.payload).unwrap_or_default()
                    )
                  })
                  .unwrap_or_default()
              } else {
                String::new()
              }
            ),
          )
        } else {
          (
            MessageRole::Error,
            format!(
              "That action was not legal: {}\n{}",
              tool.tool_name,
              tool.error.as_deref().unwrap_or("Unknown tool error")
            ),
          )
        };
        app.messages.push(ChatMessage { role, content });
      }
      if !had_visible_text && report.tool_results.is_empty() {
        app.messages.push(ChatMessage {
          role: MessageRole::Model,
          content: "I have nothing further to add for that prompt.".to_string(),
        });
      }
      app.status = format!(
        "The moment is yours • {} world change{}",
        report.tool_calls_used,
        if report.tool_calls_used == 1 { "" } else { "s" }
      );
    }
    Err(error) => {
      app.messages.push(ChatMessage {
        role: MessageRole::Error,
        content: format!("{error:#}"),
      });
      app.status = "That turn failed; the scene is still intact.".to_string();
    }
  }
  app.scroll = 0;
}

fn draw(frame: &mut Frame<'_>, app: &Playground) {
  let area = frame.area();
  if area.width < 24 || area.height < 12 {
    return;
  }
  let rows = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
      Constraint::Length(3),
      Constraint::Min(10),
      Constraint::Length(5),
      Constraint::Length(3),
    ])
    .split(area);
  frame.render_widget(
    Paragraph::new(Line::from(vec![
      Span::styled(
        " DAGR ",
        Style::default()
          .fg(Color::Black)
          .bg(Color::LightMagenta)
          .add_modifier(Modifier::BOLD),
      ),
      Span::styled(
        "  SCENE PLAYGROUND",
        Style::default()
          .fg(Color::LightMagenta)
          .add_modifier(Modifier::BOLD),
      ),
      Span::styled(
        format!("   tiny playable moments • {}", app.model),
        Style::default().fg(Color::DarkGray),
      ),
    ]))
    .block(Block::default().borders(Borders::BOTTOM)),
    rows[0],
  );
  let columns = Layout::default()
    .direction(Direction::Horizontal)
    .constraints([Constraint::Percentage(26), Constraint::Percentage(74)])
    .split(rows[1]);
  draw_actors(frame, columns[0], app);
  draw_chat(frame, columns[1], app);
  let composer = if app.input.is_empty() {
    "What do you do or say?  [F2: scene starter]▌".to_string()
  } else {
    format!("{}▌", app.input)
  };
  frame.render_widget(
    Paragraph::new(composer).wrap(Wrap { trim: false }).block(
      Block::default()
        .title(" MESSAGE ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if app.focus == Focus::Composer {
          Color::LightMagenta
        } else {
          Color::DarkGray
        })),
    ),
    rows[2],
  );
  let status = if app.searching {
    format!(" / actors: {}_", app.actor_filter)
  } else {
    format!(" {}", app.status)
  };
  frame.render_widget(Paragraph::new(Line::from(vec![Span::styled(status,Style::default().fg(Color::LightGreen)),
    Span::styled("   Tab actor/play  Enter select/send  F2 starter  F3 new scene  V details  / search  ? help  Esc back",Style::default().fg(Color::DarkGray))]))
    .block(Block::default().borders(Borders::TOP)),rows[3]);
  if app.help {
    draw_help(frame, area)
  }
}

fn draw_actors(frame: &mut Frame<'_>, area: Rect, app: &Playground) {
  let indices = app.filtered_indices();
  let items: Vec<ListItem> = indices
    .iter()
    .map(|index| {
      let entry = &app.actors[*index];
      let active = entry.actor == app.actor;
      ListItem::new(vec![
        Line::from(vec![
          Span::styled(
            if active { "● " } else { "  " },
            Style::default().fg(Color::LightGreen),
          ),
          Span::styled(&entry.name, Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::styled(&entry.summary, Style::default().fg(Color::DarkGray)),
      ])
    })
    .collect();
  let mut state = ListState::default().with_selected(
    (!items.is_empty()).then_some(app.actor_cursor.min(items.len().saturating_sub(1))),
  );
  frame.render_stateful_widget(
    List::new(items)
      .highlight_symbol("› ")
      .highlight_style(Style::default().bg(Color::Rgb(50, 25, 55)))
      .block(
        Block::default()
          .title(" ACTOR SCOPE ")
          .borders(Borders::ALL)
          .border_style(Style::default().fg(if app.focus == Focus::Actors {
            Color::LightMagenta
          } else {
            Color::DarkGray
          })),
      ),
    area,
    &mut state,
  );
}
fn draw_chat(frame: &mut Frame<'_>, area: Rect, app: &Playground) {
  let mut lines = Vec::new();
  for message in &app.messages {
    let (label, color) = match message.role {
      MessageRole::You => ("YOU", Color::LightCyan),
      MessageRole::Model => ("STORY", Color::LightMagenta),
      MessageRole::Tool => ("WORLD", Color::LightGreen),
      MessageRole::System => ("SYSTEM", Color::DarkGray),
      MessageRole::Error => ("ERROR", Color::LightRed),
    };
    lines.push(Line::styled(
      format!(" {label} "),
      Style::default()
        .fg(Color::Black)
        .bg(color)
        .add_modifier(Modifier::BOLD),
    ));
    lines.extend(
      message
        .content
        .lines()
        .map(|line| Line::raw(format!("  {line}"))),
    );
    lines.push(Line::raw(""));
  }
  let visible = area.height.saturating_sub(2);
  let approximate = lines.len().saturating_sub(visible as usize) as u16;
  let scroll = approximate.saturating_sub(app.scroll);
  frame.render_widget(
    Paragraph::new(Text::from(lines))
      .wrap(Wrap { trim: false })
      .scroll((scroll, 0))
      .block(
        Block::default()
          .title(" THE MOMENT ")
          .borders(Borders::ALL)
          .border_style(Style::default().fg(Color::Rgb(90, 45, 100))),
      ),
    area,
  );
}
fn draw_help(frame: &mut Frame<'_>, area: Rect) {
  let popup = centered(area, 64, 62);
  frame.render_widget(Clear, popup);
  let text = "SCENE PLAYGROUND\n\nTest one small piece of play: an immediate situation, a little pressure, and a decision. Choose who animates the moment on the left.\n\nF2                load/edit a scene starter\nF3                clear and begin a new moment\nTab               switch actor/play\n↑ ↓ / j k          browse actors\n/                 filter actors\nEnter             select actor or act\nV                 show/hide technical event detail\nPageUp/PageDown   scroll the scene\nCtrl-U            clear input\nEsc               clear input or return\n\nSTORY is what you experience. WORLD cards summarize committed consequences. Raw payloads stay hidden unless you ask for them with V.\n\nConfigure ANTHROPIC_API_KEY in .env. DAGR_LLM_MODEL can override Haiku.\n\nPress any key to close.";
  frame.render_widget(
    Paragraph::new(text)
      .alignment(Alignment::Center)
      .wrap(Wrap { trim: true })
      .block(
        Block::default()
          .title(" ONE SMALL ADVENTURE ")
          .borders(Borders::ALL)
          .border_style(Style::default().fg(Color::LightMagenta)),
      ),
    popup,
  );
}
fn scene_starters(actor: ActorScope) -> &'static [&'static str] {
  match actor {
    ActorScope::Gm => &[
      "Open on one tense, concrete situation connected to the most urgent campaign pressure. Give me something I can immediately do, then ask what I do.",
      "Frame a strange encounter on the road. Establish one memorable detail, hint at one danger, and stop at a meaningful choice.",
      "Put me face-to-face with someone who wants something from me right now. Keep it intimate and playable; ask how I respond.",
    ],
    ActorScope::Npc(_) => &[
      "I find you alone after something has clearly gone wrong. What do I notice, and what do you say to me?",
      "I ask for your help with the thing you are most worried about right now. Answer in character and give me a difficult reason to care.",
      "We have only a minute before someone arrives. Tell me the truth you think I need to hear—or the lie you need me to believe.",
    ],
    ActorScope::Faction(_) => &[
      "Show me one immediate sign that this faction is making its move. Put a person or place I care about under pressure and ask what I do.",
      "The faction offers me a deal that would genuinely help, but carries a visible cost. Present the offer and let me decide.",
      "Reveal the smallest dangerous consequence of this faction's current goal. Keep the scene local, concrete, and actionable.",
    ],
  }
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
  let width = area
    .width
    .saturating_mul(width)
    .saturating_div(100)
    .max(20)
    .min(area.width);
  let height = area
    .height
    .saturating_mul(height)
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
  fn playground_default_is_haiku() {
    assert!(DEFAULT_CLAUDE_MODEL.contains("haiku"));
  }

  #[test]
  fn every_actor_scope_has_small_scene_starters() {
    assert_eq!(scene_starters(ActorScope::Gm).len(), 3);
    assert!(
      scene_starters(ActorScope::Npc(dagr_lib::ids::CharacterId::new(1).unwrap()))
        .iter()
        .all(|starter| starter.len() < 220)
    );
    assert!(
      scene_starters(ActorScope::Faction(
        dagr_lib::ids::FactionId::new(1).unwrap()
      ))
      .iter()
      .all(|starter| starter.len() < 220)
    );
  }
}
