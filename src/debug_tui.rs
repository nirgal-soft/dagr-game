use std::{io, sync::Arc, time::Duration};

use anyhow::Result;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use dagr_lib::{
    Engine,
    agency::{
        ActorEventFilter, ActorEventView, ActorFilter, ActorScope, ActorSummary,
        CreativeSnapshotQuery, InvokeTool, ToolCall, ToolView,
    },
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};
use serde_json::{Map, Value, json};

struct TuiGuard;
impl Drop for TuiGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, cursor::Show);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Focus {
    Actors,
    Tools,
    Details,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum DetailView {
    Context,
    Events,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum SearchMode {
    Actors,
    Tools,
}

struct DebugApp {
    actors: Vec<ActorSummary>,
    actor_cursor: usize,
    actor_filter: String,
    actor: ActorScope,
    tools: Vec<ToolView>,
    tool_cursor: usize,
    tool_filter: String,
    context: Value,
    events: Vec<ActorEventView>,
    focus: Focus,
    detail_view: DetailView,
    detail_scroll: u16,
    search: Option<SearchMode>,
    form: Option<ToolForm>,
    help: bool,
    status: String,
    call_sequence: u64,
}

impl DebugApp {
    async fn load(engine: &Engine) -> Result<Self> {
        let actors = engine.agency().actors(ActorFilter::default()).await?;
        let actor = actors
            .first()
            .map(|entry| entry.actor)
            .unwrap_or(ActorScope::Gm);
        let tools = engine.agency().tools().await?;
        let context = actor_context(engine, actor).await?;
        let events = engine
            .agency()
            .actor_events(ActorEventFilter {
                actor: Some(actor),
                ..ActorEventFilter::default()
            })
            .await?;
        Ok(Self {
            actors,
            actor_cursor: 0,
            actor_filter: String::new(),
            actor,
            tools,
            tool_cursor: 0,
            tool_filter: String::new(),
            context,
            events,
            focus: Focus::Actors,
            detail_view: DetailView::Context,
            detail_scroll: 0,
            search: None,
            form: None,
            help: false,
            status: "Ready — choose an actor and explore their capabilities.".to_string(),
            call_sequence: 0,
        })
    }

    fn filtered_actor_indices(&self) -> Vec<usize> {
        let needle = self.actor_filter.to_ascii_lowercase();
        self.actors
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

    fn filtered_tool_indices(&self) -> Vec<usize> {
        let needle = self.tool_filter.to_ascii_lowercase();
        self.tools
            .iter()
            .enumerate()
            .filter(|(_, tool)| {
                needle.is_empty()
                    || tool.name.to_ascii_lowercase().contains(&needle)
                    || tool.description.to_ascii_lowercase().contains(&needle)
            })
            .map(|(index, _)| index)
            .collect()
    }

    async fn activate_actor(&mut self, engine: &Engine) -> Result<()> {
        let filtered = self.filtered_actor_indices();
        let Some(index) = filtered.get(self.actor_cursor).copied() else {
            return Ok(());
        };
        self.actor = self.actors[index].actor;
        self.tools = engine.agency().tools().await?;
        self.tool_cursor = 0;
        self.tool_filter.clear();
        self.refresh(engine).await?;
        self.focus = Focus::Tools;
        self.status = format!("Acting as {}", self.actors[index].name);
        Ok(())
    }

    async fn refresh(&mut self, engine: &Engine) -> Result<()> {
        self.context = actor_context(engine, self.actor).await?;
        self.events = engine
            .agency()
            .actor_events(ActorEventFilter {
                actor: Some(self.actor),
                ..ActorEventFilter::default()
            })
            .await?;
        self.detail_scroll = 0;
        Ok(())
    }

    fn selected_tool(&self) -> Option<&ToolView> {
        let filtered = self.filtered_tool_indices();
        filtered
            .get(self.tool_cursor)
            .and_then(|index| self.tools.get(*index))
    }

    fn move_selection(&mut self, delta: isize) {
        if self.focus == Focus::Details {
            self.detail_scroll = if delta < 0 {
                self.detail_scroll.saturating_sub(1)
            } else {
                self.detail_scroll.saturating_add(1)
            };
            return;
        }
        let len = if self.focus == Focus::Actors {
            self.filtered_actor_indices().len()
        } else {
            self.filtered_tool_indices().len()
        };
        let cursor = if self.focus == Focus::Actors {
            &mut self.actor_cursor
        } else {
            &mut self.tool_cursor
        };
        if len == 0 {
            *cursor = 0;
            return;
        }
        *cursor = if delta < 0 {
            cursor.saturating_sub(1)
        } else {
            (*cursor + 1).min(len - 1)
        };
    }

    fn cycle_focus(&mut self, backwards: bool) {
        self.focus = match (self.focus, backwards) {
            (Focus::Actors, false) => Focus::Tools,
            (Focus::Tools, false) => Focus::Details,
            (Focus::Details, false) => Focus::Actors,
            (Focus::Actors, true) => Focus::Details,
            (Focus::Tools, true) => Focus::Actors,
            (Focus::Details, true) => Focus::Tools,
        }
    }
}

async fn actor_context(engine: &Engine, actor: ActorScope) -> Result<Value> {
    let value = match actor {
        ActorScope::Gm => serde_json::to_value(
            engine
                .agency()
                .creative_snapshot(CreativeSnapshotQuery {
                    include_gm_only_facts: true,
                })
                .await?,
        )?,
        ActorScope::Npc(character_id) => {
            serde_json::to_value(engine.agency().npc_profile(character_id).await?)?
        }
        ActorScope::Faction(faction_id) => {
            serde_json::to_value(engine.agency().faction(faction_id).await?)?
        }
    };
    Ok(value)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DebugDestination {
    MainMenu,
    ScenePlayground,
    TagPlaytest,
}

pub async fn run(engine: Arc<Engine>) -> Result<DebugDestination> {
    terminal::enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen, cursor::Hide)?;
    let _guard = TuiGuard;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut app = DebugApp::load(&engine).await?;

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
        if app.form.is_some() {
            if handle_form_key(&mut app, key, &engine).await? {
                continue;
            }
        }
        if app.search.is_some() {
            handle_search_key(&mut app, key, &engine).await?;
            continue;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return Ok(DebugDestination::MainMenu),
            KeyCode::Char('l') | KeyCode::Char('L') => {
                return Ok(DebugDestination::ScenePlayground);
            }
            KeyCode::Char('t') | KeyCode::Char('T') => {
                return Ok(DebugDestination::TagPlaytest);
            }
            KeyCode::Char('?') => app.help = true,
            KeyCode::Char('/') => {
                app.search = Some(if app.focus == Focus::Actors {
                    SearchMode::Actors
                } else {
                    SearchMode::Tools
                });
            }
            KeyCode::Tab => app.cycle_focus(key.modifiers.contains(KeyModifiers::SHIFT)),
            KeyCode::BackTab => app.cycle_focus(true),
            KeyCode::Up | KeyCode::Char('k') => app.move_selection(-1),
            KeyCode::Down | KeyCode::Char('j') => app.move_selection(1),
            KeyCode::PageUp => app.detail_scroll = app.detail_scroll.saturating_sub(10),
            KeyCode::PageDown => app.detail_scroll = app.detail_scroll.saturating_add(10),
            KeyCode::Char('e') => {
                app.detail_view = if app.detail_view == DetailView::Context {
                    DetailView::Events
                } else {
                    DetailView::Context
                };
                app.detail_scroll = 0;
            }
            KeyCode::Char('d') => match crate::debug_scenario::create(engine.clone()).await {
                Ok(scenario) => {
                    app.actors = engine.agency().actors(ActorFilter::default()).await?;
                    app.status = format!(
                        "Demo ready — NPC {}, target {}, Faction {}, Danger {}",
                        scenario.npc_id,
                        scenario.target_id,
                        scenario.faction_id,
                        scenario.danger_id
                    );
                }
                Err(error) => app.status = format!("Could not create demo: {error:#}"),
            },
            KeyCode::Char('r') => match app.refresh(&engine).await {
                Ok(()) => app.status = "Canonical state refreshed.".to_string(),
                Err(error) => app.status = format!("Refresh failed: {error:#}"),
            },
            KeyCode::Enter => match app.focus {
                Focus::Actors => {
                    if let Err(error) = app.activate_actor(&engine).await {
                        app.status = format!("Could not select actor: {error:#}")
                    }
                }
                Focus::Tools => {
                    if let Some(tool) = app.selected_tool().cloned() {
                        app.form = Some(ToolForm::new(tool));
                    }
                }
                Focus::Details => {
                    app.detail_view = if app.detail_view == DetailView::Context {
                        DetailView::Events
                    } else {
                        DetailView::Context
                    }
                }
            },
            _ => {}
        }
    }
}

async fn handle_search_key(app: &mut DebugApp, key: KeyEvent, engine: &Engine) -> Result<()> {
    let mode = app.search.expect("search mode");
    let query = if mode == SearchMode::Actors {
        &mut app.actor_filter
    } else {
        &mut app.tool_filter
    };
    match key.code {
        KeyCode::Esc => {
            query.clear();
            app.search = None;
        }
        KeyCode::Enter => {
            app.search = None;
            if mode == SearchMode::Actors {
                app.activate_actor(engine).await?;
            } else if let Some(tool) = app.selected_tool().cloned() {
                app.form = Some(ToolForm::new(tool));
            }
        }
        KeyCode::Backspace => {
            query.pop();
            if mode == SearchMode::Actors {
                app.actor_cursor = 0
            } else {
                app.tool_cursor = 0
            }
        }
        KeyCode::Char(character) => {
            query.push(character);
            if mode == SearchMode::Actors {
                app.actor_cursor = 0
            } else {
                app.tool_cursor = 0
            }
        }
        KeyCode::Up => app.move_selection(-1),
        KeyCode::Down => app.move_selection(1),
        _ => {}
    }
    Ok(())
}

async fn handle_form_key(app: &mut DebugApp, key: KeyEvent, engine: &Engine) -> Result<bool> {
    match key.code {
        KeyCode::Esc => {
            app.form = None;
            app.status = "Tool invocation cancelled.".to_string();
        }
        KeyCode::F(2) => {
            if let Some(form) = app.form.as_mut() {
                form.load_example()
            }
        }
        KeyCode::F(5) => {
            let (tool_name, input) = match app.form.as_ref().expect("form").build() {
                Ok(value) => value,
                Err(error) => {
                    app.form.as_mut().unwrap().error = Some(error);
                    return Ok(true);
                }
            };
            app.call_sequence += 1;
            let result = engine
                .agency()
                .invoke_tool(InvokeTool {
                    actor: app.actor,
                    call: ToolCall {
                        id: format!("tui-{}-{}", std::process::id(), app.call_sequence),
                        name: tool_name,
                        input,
                    },
                })
                .await?;
            if result.success {
                app.status = format!(
                    "✓ {} committed as event #{}",
                    result.tool_name,
                    result
                        .event
                        .as_ref()
                        .map(|event| event.event_id.to_string())
                        .unwrap_or_default()
                );
                app.form = None;
                app.detail_view = DetailView::Events;
                app.refresh(engine).await?;
            } else {
                app.form.as_mut().unwrap().error = result.error;
            }
        }
        KeyCode::Tab | KeyCode::Down => app.form.as_mut().unwrap().next(),
        KeyCode::BackTab | KeyCode::Up => app.form.as_mut().unwrap().previous(),
        KeyCode::Left => app.form.as_mut().unwrap().cycle_enum(-1),
        KeyCode::Right => app.form.as_mut().unwrap().cycle_enum(1),
        KeyCode::Backspace => app.form.as_mut().unwrap().backspace(),
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.form.as_mut().unwrap().clear()
        }
        KeyCode::Char(character) => app.form.as_mut().unwrap().push(character),
        _ => {}
    }
    Ok(true)
}

struct FormField {
    name: String,
    kind: String,
    description: String,
    required: bool,
    value: String,
    choices: Vec<String>,
}

struct ToolForm {
    tool: ToolView,
    fields: Vec<FormField>,
    selected: usize,
    error: Option<String>,
}

impl ToolForm {
    fn new(tool: ToolView) -> Self {
        let required: Vec<&str> = tool
            .input_schema
            .get("required")
            .and_then(Value::as_array)
            .map(|values| values.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        let fields = tool
            .input_schema
            .get("properties")
            .and_then(Value::as_object)
            .map(|properties| {
                properties
                    .iter()
                    .map(|(name, schema)| {
                        let kind = schema
                            .get("type")
                            .and_then(Value::as_str)
                            .unwrap_or("json")
                            .to_string();
                        let choices = schema
                            .get("enum")
                            .and_then(Value::as_array)
                            .map(|values| {
                                values
                                    .iter()
                                    .filter_map(Value::as_str)
                                    .map(str::to_string)
                                    .collect()
                            })
                            .unwrap_or_default();
                        let value = schema
                            .get("default")
                            .map(display_default)
                            .unwrap_or_default();
                        FormField {
                            name: name.clone(),
                            kind,
                            description: schema
                                .get("description")
                                .and_then(Value::as_str)
                                .unwrap_or("")
                                .to_string(),
                            required: required.contains(&name.as_str()),
                            value,
                            choices,
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();
        Self {
            tool,
            fields,
            selected: 0,
            error: None,
        }
    }
    fn next(&mut self) {
        if !self.fields.is_empty() {
            self.selected = (self.selected + 1) % self.fields.len()
        }
    }
    fn previous(&mut self) {
        if !self.fields.is_empty() {
            self.selected = if self.selected == 0 {
                self.fields.len() - 1
            } else {
                self.selected - 1
            }
        }
    }
    fn push(&mut self, character: char) {
        if let Some(field) = self.fields.get_mut(self.selected) {
            field.value.push(character);
            self.error = None
        }
    }
    fn backspace(&mut self) {
        if let Some(field) = self.fields.get_mut(self.selected) {
            field.value.pop();
            self.error = None
        }
    }
    fn clear(&mut self) {
        if let Some(field) = self.fields.get_mut(self.selected) {
            field.value.clear();
            self.error = None
        }
    }
    fn cycle_enum(&mut self, delta: isize) {
        let Some(field) = self.fields.get_mut(self.selected) else {
            return;
        };
        if field.choices.is_empty() {
            return;
        }
        let current = field
            .choices
            .iter()
            .position(|choice| choice == &field.value)
            .unwrap_or(0);
        let next = if delta < 0 {
            if current == 0 {
                field.choices.len() - 1
            } else {
                current - 1
            }
        } else {
            (current + 1) % field.choices.len()
        };
        field.value = field.choices[next].clone();
    }
    fn load_example(&mut self) {
        if let Some(example) = tool_example(&self.tool.name) {
            for field in &mut self.fields {
                if let Some(value) = example.get(&field.name) {
                    field.value = display_default(value)
                }
            }
            self.error = None;
        }
    }
    fn build(&self) -> std::result::Result<(String, Value), String> {
        let mut object = Map::new();
        for field in &self.fields {
            if field.value.trim().is_empty() {
                if field.required {
                    return Err(format!("{} is required", field.name));
                }
                continue;
            }
            let value =
                parse_form_value(field).map_err(|error| format!("{}: {error}", field.name))?;
            object.insert(field.name.clone(), value);
        }
        Ok((self.tool.name.to_string(), Value::Object(object)))
    }
}

fn parse_form_value(field: &FormField) -> std::result::Result<Value, String> {
    match field.kind.as_str() {
        "string" => Ok(Value::String(field.value.clone())),
        "integer" => field
            .value
            .trim()
            .parse::<i64>()
            .map(Value::from)
            .map_err(|_| "enter a whole number".to_string()),
        "number" => field
            .value
            .trim()
            .parse::<f64>()
            .map(Value::from)
            .map_err(|_| "enter a number".to_string()),
        "boolean" => field
            .value
            .trim()
            .parse::<bool>()
            .map(Value::Bool)
            .map_err(|_| "enter true or false".to_string()),
        _ => serde_json::from_str(&field.value)
            .map_err(|error| format!("enter valid JSON ({error})")),
    }
}
fn display_default(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        other => other.to_string(),
    }
}

fn tool_example(name: &str) -> Option<Value> {
    Some(match name {
        "npc_speak" => json!({"dialogue":"Keep your voice down. The road has ears."}),
        "npc_adjust_relationship" => {
            json!({"target_character_id":1,"change":10,"reason":"They kept their promise"})
        }
        "npc_attempt_action" => {
            json!({"description":"hides the map beneath the ledgers","intent":"conceal the route"})
        }
        "faction_attempt_move" => {
            json!({"description":"closes the eastern road","objective":"force the clans to negotiate"})
        }
        "front_advance" => json!({"danger_id":1,"reason":"The threat was left unanswered"}),
        "gm_frame_scene" => {
            json!({"title":"Ash at the Gate","framing":"Funeral ash falls upward as the caravan reaches the sealed gate.","tension":"The wardens are approaching and the gate demands a name.","participants":[]})
        }
        "gm_apply_effects" => {
            json!({"summary":"Make the impossible gate matter","effects":[{"type":"establish_fact","subject":"The eastern gate","assertion":"It opens only for someone carrying funeral ash","visibility":"public"},{"type":"offer_decision","prompt":"Who carries the ash?","options":["Amleth","Beatrice"],"stakes":"The gate marks whoever enters first"}]})
        }
        "gm_introduce_danger" => {
            json!({"front_id":1,"faction_id":null,"category":"Arcane Enemy","name":"The Road That Hungers","impulse":"To lead travelers away from home","impending_doom":"Destruction","grim_portents":["Milestones begin changing names","The eastern road vanishes from maps"],"moves":["Offer a convincing shortcut"]})
        }
        "gm_resolve_attempt" => {
            json!({"attempt_kind":"npc_action","attempt_id":1,"outcome":"partial_success","narration":"They succeed, but draw dangerous attention.","effects":[]})
        }
        "gm_resolve_decision" => {
            json!({"decision_id":1,"selected_option":"Amleth","consequence_summary":"Amleth accepts the mark.","effects":[]})
        }
        _ => return None,
    })
}

fn draw(frame: &mut Frame<'_>, app: &DebugApp) {
    let area = frame.area();
    if area.width < 20 || area.height < 8 {
        return;
    }
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(area);
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            " DAGR ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "  ENGINE WORKBENCH",
            Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("   deterministic state • creative authority   "),
        Span::styled(
            "[ L  SCENE PLAYGROUND ]  [ T  TAG PLAYTEST ]",
            Style::default()
                .fg(Color::LightMagenta)
                .add_modifier(Modifier::BOLD),
        ),
    ]))
    .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(header, rows[0]);

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(34),
            Constraint::Percentage(41),
        ])
        .split(rows[1]);
    draw_actors(frame, columns[0], app);
    draw_tools(frame, columns[1], app);
    draw_details(frame, columns[2], app);

    let search = match app.search {
        Some(SearchMode::Actors) => format!("  / actors: {}_", app.actor_filter),
        Some(SearchMode::Tools) => format!("  / tools: {}_", app.tool_filter),
        None => format!("  {}", app.status),
    };
    let footer=Paragraph::new(Line::from(vec![
    Span::styled(search,Style::default().fg(Color::LightGreen)),
    Span::styled("   Tab panes  ↑↓ navigate  Enter select  / search  e context/events  L playground  d demo  r refresh  ? help  q back ",Style::default().fg(Color::DarkGray)),
  ])).block(Block::default().borders(Borders::TOP));
    frame.render_widget(footer, rows[2]);
    if let Some(form) = &app.form {
        draw_form(frame, area, form)
    }
    if app.help {
        draw_help(frame, area)
    }
}

fn panel_block(title: impl Into<String>, focused: bool) -> Block<'static> {
    Block::default()
        .title(format!(" {} ", title.into()))
        .borders(Borders::ALL)
        .border_style(if focused {
            Style::default().fg(Color::LightCyan)
        } else {
            Style::default().fg(Color::DarkGray)
        })
}
fn draw_actors(frame: &mut Frame<'_>, area: Rect, app: &DebugApp) {
    let indices = app.filtered_actor_indices();
    let items: Vec<ListItem> = indices
        .iter()
        .map(|index| {
            let actor = &app.actors[*index];
            ListItem::new(vec![
                Line::styled(&actor.name, Style::default().add_modifier(Modifier::BOLD)),
                Line::styled(&actor.summary, Style::default().fg(Color::DarkGray)),
            ])
        })
        .collect();
    let mut state = ListState::default().with_selected(
        (!items.is_empty()).then_some(app.actor_cursor.min(items.len().saturating_sub(1))),
    );
    let list = List::new(items)
        .block(panel_block(
            &format!(
                "ACTORS  {}",
                if app.actor_filter.is_empty() {
                    ""
                } else {
                    "(filtered)"
                }
            ),
            app.focus == Focus::Actors,
        ))
        .highlight_style(Style::default().bg(Color::Rgb(25, 55, 65)).fg(Color::White))
        .highlight_symbol("› ");
    frame.render_stateful_widget(list, area, &mut state);
}
fn draw_tools(frame: &mut Frame<'_>, area: Rect, app: &DebugApp) {
    let indices = app.filtered_tool_indices();
    let items: Vec<ListItem> = indices
        .iter()
        .map(|index| {
            let tool = &app.tools[*index];
            ListItem::new(vec![
                Line::styled(
                    tool.name.as_str(),
                    Style::default()
                        .fg(Color::LightYellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Line::styled(tool.description.as_str(), Style::default().fg(Color::Gray)),
            ])
        })
        .collect();
    let mut state = ListState::default().with_selected(
        (!items.is_empty()).then_some(app.tool_cursor.min(items.len().saturating_sub(1))),
    );
    let list = List::new(items)
        .block(panel_block("CAPABILITIES", app.focus == Focus::Tools))
        .highlight_style(Style::default().bg(Color::Rgb(60, 50, 20)).fg(Color::White))
        .highlight_symbol("› ");
    frame.render_stateful_widget(list, area, &mut state);
}
fn draw_details(frame: &mut Frame<'_>, area: Rect, app: &DebugApp) {
    let (title, text) = if app.detail_view == DetailView::Context {
        (
            "CANONICAL CONTEXT  [e: events]",
            serde_json::to_string_pretty(&app.context).unwrap_or_default(),
        )
    } else {
        ("EVENT TIMELINE  [e: context]", format_events(&app.events))
    };
    let paragraph = Paragraph::new(text)
        .block(panel_block(title, app.focus == Focus::Details))
        .wrap(Wrap { trim: false })
        .scroll((app.detail_scroll, 0));
    frame.render_widget(paragraph, area);
}
fn format_events(events: &[ActorEventView]) -> String {
    if events.is_empty() {
        return "No events yet.\n\nSuccessful tool calls appear here as durable consequences."
            .to_string();
    }
    events
        .iter()
        .rev()
        .take(20)
        .map(|event| {
            format!(
                "● #{}  {}\n{}",
                event.event_id,
                event.event_type,
                serde_json::to_string_pretty(&event.payload).unwrap_or_default()
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}
fn draw_form(frame: &mut Frame<'_>, area: Rect, form: &ToolForm) {
    let popup = centered(area, 82, 82);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(format!(" INVOKE {} ", form.tool.name))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::LightMagenta));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Percentage(48),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(inner);
    frame.render_widget(
        Paragraph::new(form.tool.description.as_str()).style(Style::default().fg(Color::Gray)),
        rows[0],
    );
    let items: Vec<ListItem> = form
        .fields
        .iter()
        .map(|field| {
            let marker = if field.required { "*" } else { " " };
            let shown = if field.value.is_empty() {
                "‹empty›".to_string()
            } else {
                truncate(&field.value, 72)
            };
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{marker} {:<22}", field.name),
                    Style::default().fg(Color::LightCyan),
                ),
                Span::raw(shown),
            ]))
        })
        .collect();
    let mut state =
        ListState::default().with_selected((!items.is_empty()).then_some(form.selected));
    frame.render_stateful_widget(
        List::new(items)
            .highlight_symbol("› ")
            .highlight_style(Style::default().bg(Color::Rgb(45, 30, 55))),
        rows[1],
        &mut state,
    );
    let editor = if let Some(field) = form.fields.get(form.selected) {
        format!(
            "{} ({}){}\n{}\n\n{}▌",
            field.name,
            field.kind,
            if field.required { " — required" } else { "" },
            field.description,
            field.value
        )
    } else {
        "This tool has no arguments.".to_string()
    };
    frame.render_widget(
        Paragraph::new(editor)
            .block(
                Block::default()
                    .title(" FIELD EDITOR ")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
        rows[2],
    );
    let message = form.error.as_deref().unwrap_or(
        "Tab/↑↓ field  type to edit  ←→ enum  Ctrl-U clear  F2 example  F5 commit  Esc cancel",
    );
    frame.render_widget(
        Paragraph::new(message).style(Style::default().fg(if form.error.is_some() {
            Color::LightRed
        } else {
            Color::DarkGray
        })),
        rows[3],
    );
}
fn draw_help(frame: &mut Frame<'_>, area: Rect) {
    let popup = centered(area, 66, 62);
    frame.render_widget(Clear, popup);
    let help = "JOYFUL ENGINE WORKBENCH\n\nTab / Shift-Tab   move between panes\n↑ ↓ or j k         navigate\nEnter              select actor or open tool\n/                  live filter / autocomplete\ne                  toggle context and event timeline\nL                  open a tiny AI-powered gameplay scene\nd                  create a demo campaign scenario\nr                  refresh canonical state\nPageUp / PageDown  scroll details\n\nTOOL FORM\nTab / ↑ ↓          move fields\nType / Backspace   edit without losing other fields\n← →                 cycle enum choices\nCtrl-U              clear selected field\nF2                  load an editable example\nF5                  validate and commit\nEsc                 cancel without losing engine state\n\nPress any key to close help.";
    frame.render_widget(
        Paragraph::new(help)
            .block(Block::default().title(" HELP ").borders(Borders::ALL))
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: false }),
        popup,
    );
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
        x: area.x + (area.width.saturating_sub(width)) / 2,
        y: area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    }
}
fn truncate(value: &str, max: usize) -> String {
    let mut text: String = value.chars().take(max).collect();
    if value.chars().count() > max {
        text.push('…')
    }
    text
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn form_examples_produce_valid_objects() {
        for name in [
            "npc_speak",
            "gm_apply_effects",
            "gm_frame_scene",
            "gm_introduce_danger",
        ] {
            assert!(tool_example(name).unwrap().is_object());
        }
    }
    #[test]
    fn form_value_parsing_is_typed() {
        let integer = FormField {
            name: "id".into(),
            kind: "integer".into(),
            description: String::new(),
            required: true,
            value: "12".into(),
            choices: Vec::new(),
        };
        assert_eq!(parse_form_value(&integer).unwrap(), Value::from(12));
    }
}
