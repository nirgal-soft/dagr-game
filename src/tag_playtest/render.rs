use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use super::TagPlaytest;

impl TagPlaytest {
    pub(super) fn draw(&self, frame: &mut Frame<'_>) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(4),
            ])
            .split(frame.area());
        frame.render_widget(
            Paragraph::new("TAG WORKBENCH  browse → draw → apply/replace → contribute → reopen")
                .block(Block::default().borders(Borders::ALL)),
            rows[0],
        );
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(35),
                Constraint::Percentage(40),
            ])
            .split(rows[1]);
        self.draw_catalogue(frame, columns[0]);
        self.draw_definitions_and_carriers(frame, columns[1]);
        self.draw_activity(frame, columns[2]);
        frame.render_widget(
      Paragraph::new(format!(
        "Tab pane  ↑/↓ select  d draw set  p apply  x replace  a accept hook  i reinstall  r refresh  q back\n[{:?}] {}",
        self.focus, self.status
      ))
      .block(Block::default().borders(Borders::ALL))
      .wrap(Wrap { trim: true }),
      rows[2],
    );
    }

    fn draw_catalogue(&self, frame: &mut Frame<'_>, area: Rect) {
        let lines = self
            .categories
            .iter()
            .enumerate()
            .map(|(index, category)| {
                let marker = if index == self.category_index {
                    ">"
                } else {
                    " "
                };
                Line::from(format!(
                    "{marker} {} [{:?}] {}@{}",
                    category.name, category.subject, category.source_pack, category.pack_version
                ))
                .style(if index == self.category_index {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                })
            })
            .collect::<Vec<_>>();
        frame.render_widget(
            Paragraph::new(lines)
                .block(Block::default().title("Catalogue").borders(Borders::ALL))
                .wrap(Wrap { trim: false }),
            area,
        );
    }

    fn draw_definitions_and_carriers(&self, frame: &mut Frame<'_>, area: Rect) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
            .split(area);
        let definitions = self
            .definitions
            .iter()
            .map(|definition| {
                Line::from(format!(
                    "{} — {} (weight {}, {}@{})",
                    definition.name,
                    definition.concept,
                    definition.weight,
                    definition.source_pack,
                    definition.pack_version
                ))
            })
            .collect::<Vec<_>>();
        frame.render_widget(
            Paragraph::new(definitions)
                .block(
                    Block::default()
                        .title(format!("Definitions ({})", self.definitions.len()))
                        .borders(Borders::ALL),
                )
                .wrap(Wrap { trim: false }),
            rows[0],
        );
        let carriers = self
            .npcs
            .iter()
            .enumerate()
            .map(|(index, npc)| {
                let marker = if index == self.npc_index { ">" } else { " " };
                Line::from(format!("{marker} {} (NPC {})", npc.name, npc.id))
            })
            .collect::<Vec<_>>();
        frame.render_widget(
            Paragraph::new(carriers)
                .block(Block::default().title("Carriers").borders(Borders::ALL)),
            rows[1],
        );
    }

    fn draw_activity(&self, frame: &mut Frame<'_>, area: Rect) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Percentage(30),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
            ])
            .split(area);
        self.draw_tag_sets(frame, rows[0]);
        self.draw_applications(frame, rows[1]);
        let hooks = self
            .hooks
            .iter()
            .enumerate()
            .map(|(index, hook)| {
                let marker = if index == self.hook_index { ">" } else { " " };
                Line::from(format!(
                    "{marker} {} / {} — {} [Application {}]",
                    hook.definition.name, hook.role.key, hook.prompt.text, hook.application.id
                ))
            })
            .collect::<Vec<_>>();
        frame.render_widget(
            Paragraph::new(hooks)
                .block(
                    Block::default()
                        .title(format!("Candidate Hooks ({})", self.hooks.len()))
                        .borders(Borders::ALL),
                )
                .wrap(Wrap { trim: false }),
            rows[2],
        );
        let mut dangers = Vec::new();
        for (index, danger) in self.dangers.iter().enumerate() {
            let marker = if index == self.danger_index { ">" } else { " " };
            dangers.push(Line::from(format!(
                "{marker} {} / {} ({:?})",
                danger.front_name, danger.danger_name, danger.artifact
            )));
            for contribution in self
                .contributions
                .iter()
                .filter(|contribution| contribution.artifact == danger.artifact)
            {
                dangers.push(Line::from(format!(
                    "  Contribution to {} · Application {} · {}@{} · {} / {}",
                    danger.danger_name,
                    contribution.hook.application.id,
                    contribution.hook.definition.source_pack,
                    contribution.hook.definition.pack_version,
                    contribution.hook.role.key,
                    contribution.hook.prompt.text
                )));
            }
        }
        frame.render_widget(
            Paragraph::new(dangers)
                .block(
                    Block::default()
                        .title(format!("Existing Dangers ({})", self.dangers.len()))
                        .borders(Borders::ALL),
                )
                .wrap(Wrap { trim: false }),
            rows[3],
        );
    }

    fn draw_tag_sets(&self, frame: &mut Frame<'_>, area: Rect) {
        let mut lines = Vec::new();
        for (index, tag_set) in self.tag_sets.iter().enumerate() {
            let selected = index == self.tag_set_index;
            lines.push(
                Line::from(format!(
                    "{} Set {} · {} · seed {} · {}@{}",
                    if selected { ">" } else { " " },
                    tag_set.id,
                    tag_set.provenance.category.name,
                    tag_set.provenance.seed,
                    tag_set.provenance.category.source_pack,
                    tag_set.provenance.category.pack_version
                ))
                .style(if selected {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                }),
            );
            if selected {
                for member in &tag_set.members {
                    lines.push(Line::from(format!(
                        "  {} — {} [{}@{}]",
                        member.name, member.concept, member.source_pack, member.pack_version
                    )));
                }
            }
        }
        frame.render_widget(
            Paragraph::new(Text::from(lines))
                .block(
                    Block::default()
                        .title(format!("Frozen Tag Sets ({})", self.tag_sets.len()))
                        .borders(Borders::ALL),
                )
                .wrap(Wrap { trim: false }),
            area,
        );
    }

    fn draw_applications(&self, frame: &mut Frame<'_>, area: Rect) {
        let mut lines = Vec::new();
        for applied in &self.applications {
            lines.push(Line::from(format!(
                "Application {} · Set {} · seed {} · {}@{}",
                applied.application.id,
                applied.tag_set.id,
                applied.tag_set.provenance.seed,
                applied.tag_set.provenance.category.source_pack,
                applied.tag_set.provenance.category.pack_version
            )));
            for member in &applied.tag_set.members {
                lines.push(Line::from(format!(
                    "  {} — {}",
                    member.name, member.concept
                )));
            }
        }
        frame.render_widget(
            Paragraph::new(Text::from(lines))
                .block(
                    Block::default()
                        .title(format!("Applications ({})", self.applications.len()))
                        .borders(Borders::ALL),
                )
                .wrap(Wrap { trim: false }),
            area,
        );
    }
}
