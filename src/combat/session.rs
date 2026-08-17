use std::sync::Arc;

use anyhow::{Context, Result};
use dagr_lib::{
  Engine, OccupantsQuery, RelocateCharacter,
  characters::{CharacterFilter, CharacterId, CharacterKind, CreatePlayer},
  combat::{EnemyAction, EnemyTurnRequest, ResetArenaRequest, StrikeOutcome, StrikeRequest},
  content::{CharacterLevel, ContentKey, DefinitionFilter},
  world::LocationId,
};

use super::{
  log::CombatLog,
  picker::{MonsterChoice, MonsterPicker},
};

const ARENA_FIGHTER_NAME: &str = "Arena Fighter";

#[derive(Clone, Debug)]
pub struct EnemyAtTile {
  pub id: CharacterId,
  pub name: String,
  pub x: i32,
  pub y: i32,
}

pub struct CombatSession {
  engine: Arc<Engine>,
  player: Option<CharacterId>,
  player_hp: Option<(i32, i32)>,
  enemies: Vec<EnemyAtTile>,
  picker: Option<MonsterPicker>,
  log: CombatLog,
}

impl CombatSession {
  pub fn new(engine: Arc<Engine>) -> Self {
    Self {
      engine,
      player: None,
      player_hp: None,
      enemies: Vec::new(),
      picker: None,
      log: CombatLog::default(),
    }
  }

  pub fn player(&self) -> Option<CharacterId> {
    self.player
  }

  pub fn picker(&self) -> Option<&MonsterPicker> {
    self.picker.as_ref()
  }

  pub fn picker_is_open(&self) -> bool {
    self.picker.is_some()
  }

  pub fn log_lines(&self, count: usize) -> Vec<String> {
    self.log.recent(count)
  }

  pub fn log(&mut self, line: impl Into<String>) {
    self.log.push(line)
  }

  pub fn hit_points(&self) -> Option<(i32, i32)> {
    self.player_hp
  }

  pub fn enemy_at(&self, x: i32, y: i32) -> Option<&EnemyAtTile> {
    self
      .enemies
      .iter()
      .find(|enemy| enemy.x == x && enemy.y == y)
  }

  pub fn enemy_positions(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
    self.enemies.iter().map(|enemy| (enemy.x, enemy.y))
  }

  pub async fn enter_arena(
    &mut self,
    location: LocationId,
    spawn: (i32, i32),
  ) -> Result<(i32, i32)> {
    let existing = self
      .engine
      .characters()
      .list(CharacterFilter {
        kind: Some(CharacterKind::Player),
        ..CharacterFilter::default()
      })
      .await?
      .into_iter()
      .find(|character| character.name == ARENA_FIGHTER_NAME);
    let player = match existing {
      Some(character) => {
        self
          .engine
          .relocate(RelocateCharacter {
            character: character.id,
            location,
            x: spawn.0,
            y: spawn.1,
          })
          .await?;
        character.id
      }
      None => {
        self
          .engine
          .spawn_player(dagr_lib::SpawnPlayer {
            character: CreatePlayer {
              name: ARENA_FIGHTER_NAME.to_string(),
              class: ContentKey::new("core:strong")?,
              level: CharacterLevel::new(5)?,
              seed: 5,
            },
            location,
            x: spawn.0,
            y: spawn.1,
          })
          .await?
          .character
          .id
      }
    };
    self.player = Some(player);
    self
      .engine
      .reset_arena(ResetArenaRequest { player })
      .await?;
    self.refresh_player().await?;
    self.refresh_enemies(location).await?;
    Ok(spawn)
  }

  pub async fn player_attack(&mut self, enemy: CharacterId) -> Result<StrikeOutcome> {
    let player = self.player.context("combat arena player is unavailable")?;
    let outcome = self
      .engine
      .strike(StrikeRequest {
        attacker: player,
        defender: enemy,
      })
      .await?;
    self
      .log
      .record_player_attack(&outcome.defender_name, &outcome);
    self.refresh_player().await?;
    Ok(outcome)
  }

  pub async fn advance_enemies(&mut self, location: LocationId) -> Result<bool> {
    let Some(player) = self.player else {
      return Ok(false);
    };
    let outcome = self
      .engine
      .advance_enemy_turns(EnemyTurnRequest { player })
      .await?;
    let acted = !outcome.actions.is_empty();
    for action in outcome.actions {
      if let EnemyAction::Attack { strike } = action {
        self.log.record_enemy_attack(&strike.attacker_name, &strike);
      }
    }
    self.refresh_player().await?;
    self.refresh_enemies(location).await?;
    Ok(acted)
  }

  pub async fn reset_arena(&mut self, location: LocationId) -> Result<()> {
    if let Some(player) = self.player {
      self
        .engine
        .reset_arena(ResetArenaRequest { player })
        .await?;
      self.refresh_player().await?;
      self.refresh_enemies(location).await?;
      self.log.clear();
      self
        .log
        .push("Arena reset: fighter restored, opponents cleared.");
    }
    Ok(())
  }

  pub async fn refresh_enemies(&mut self, location: LocationId) -> Result<()> {
    self.enemies = self
      .engine
      .occupants(OccupantsQuery {
        location,
        after_character: None,
        limit: OccupantsQuery::MAX_LIMIT,
      })
      .await?
      .into_iter()
      .filter(|occupant| occupant.kind == CharacterKind::Monster)
      .map(|occupant| EnemyAtTile {
        id: occupant.character,
        name: occupant.name,
        x: occupant.placement.x,
        y: occupant.placement.y,
      })
      .collect();
    Ok(())
  }

  pub async fn open_picker(&mut self) -> Result<()> {
    let definitions = self
      .engine
      .content()
      .monsters(DefinitionFilter::default())
      .await?;
    self.picker = Some(MonsterPicker::new(definitions));
    Ok(())
  }

  pub fn close_picker(&mut self) {
    self.picker = None
  }

  pub fn picker_input(&mut self, character: char) {
    if let Some(picker) = self.picker.as_mut() {
      picker.input(character)
    }
  }

  pub fn picker_backspace(&mut self) {
    if let Some(picker) = self.picker.as_mut() {
      picker.backspace()
    }
  }

  pub fn picker_move(&mut self, delta: i32) {
    if let Some(picker) = self.picker.as_mut() {
      picker.move_selection(delta)
    }
  }

  pub fn selected_monster(&self) -> Option<MonsterChoice> {
    self.picker.as_ref()?.selected().cloned()
  }

  async fn refresh_player(&mut self) -> Result<()> {
    let Some(player) = self.player else {
      self.player_hp = None;
      return Ok(());
    };
    let character = self.engine.characters().character(player).await?;
    self.player_hp = Some((character.base_stats.current_hp, character.base_stats.max_hp));
    Ok(())
  }
}
