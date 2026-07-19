use anyhow::{Result, anyhow};
use dagr_lib::components::{
  stats::base_stats::BaseStatsData,
  world::{hex::Hex, location::Location, spatial::Spatial, wilderness::Wilderness},
};
use dagr_lib::ems::{component::Component, entity_manager::EntityManager};
use dagr_lib::factories::{
  characters::character::{CharacterPositionSeed, MonsterCharacterSeed},
  world::{
    dungeon::DungeonSeed,
    hex::HexSeed,
    wilderness::WildernessAreaSeed,
  },
};
use dagr_lib::ids::LocationId;
use crossterm::style::Color;
use tracing::info;

use crate::camera::Camera;
use crate::combat::{controller, picker::MonsterChoice, session::CombatSession};
use crate::generators::arena::{
  COMBAT_ARENA_HEIGHT, COMBAT_ARENA_KEY, COMBAT_ARENA_WIDTH,
};
use crate::inspection::{mode::LookMode, resolver};
use crate::navigation::Navigator;
use crate::renderer::{RenderConfig, Tile};
use crate::views::{
  Transition, TransitionIntent, TransitionOutcome, ViewManager, WildernessCrossing,
};
use crate::wilderness_layout::WildernessLayout;
use crate::world_map::WorldMap;

pub struct GameState {
  pub entity_manager: EntityManager,
  pub map: WorldMap,
  pub camera: Camera,
  pub player_x: i32,
  pub player_y: i32,
  pub view_manager: ViewManager,
  pub render_config: RenderConfig,
  navigator: Navigator,
  pub combat: CombatSession,
  look_mode: Option<LookMode>,
  pub popup_message: Option<String>,
}

impl GameState {
  pub fn new(
    entity_manager: EntityManager,
    pool: std::sync::Arc<sqlx::PgPool>,
    view_w: u16,
    view_h: u16,
    wilderness_layout: WildernessLayout,
  ) -> Self {
    let mut state = Self {
      entity_manager,
      map: WorldMap::new(),
      camera: Camera::new(view_w, view_h),
      player_x: 0,
      player_y: 0,
      view_manager: ViewManager::new(wilderness_layout),
      render_config: RenderConfig::default(),
      navigator: Navigator::new(),
      combat: CombatSession::new(pool),
      look_mode: None,
      popup_message: None,
    };
    state.rebuild_map();
    state.attach_tiles();
    state
  }

  pub fn is_in_world(&self) -> bool {
    self.view_manager.is_in_world()
  }

  pub fn current_view_label(&self) -> String {
    if self.view_manager.is_in_world() {
      "World".to_string()
    } else if let Some(level) = self.view_manager.current_level() {
      format!("Dungeon level {level}")
    } else if self.is_combat_arena() {
      "Combat arena".to_string()
    } else {
      "Wilderness".to_string()
    }
  }

  fn is_combat_arena(&self) -> bool {
    self.view_manager.current_entity()
      .and_then(|entity| self.entity_manager.get_component::<Wilderness, _>(entity).ok())
      .is_some_and(|wilderness| wilderness.get().get_area_key() == COMBAT_ARENA_KEY)
  }

  pub fn coordinate_debug_lines(&self) -> Vec<String> {
    if self.view_manager.is_in_world() {
      return vec![
        format!("hex: ({}, {})", self.player_x, self.player_y),
        "area: world map".to_string(),
        format!("tile: ({}, {})", self.player_x, self.player_y),
      ]
    }

    let Some(entity) = self.view_manager.current_entity() else{
      return vec![format!("tile: ({}, {})", self.player_x, self.player_y)]
    };
    let area = self.entity_manager.get_component::<Spatial, _>(entity).ok().map(|spatial| spatial.get());
    let hex_coords = self.entity_manager.get_component::<Location, _>(entity).ok()
      .and_then(|location| location.get().get_parent_location_id().ok().flatten())
      .and_then(|parent| self.entity_manager.find_entity_by_location_id::<Hex>(parent))
      .and_then(|hex| self.entity_manager.get_component::<Spatial, _>(hex).ok())
      .map(|spatial| {
        let spatial = spatial.get();
        (spatial.get_x(), spatial.get_y())
      });
    vec![
      hex_coords.map(|(x, y)| format!("hex: ({x}, {y})"))
        .unwrap_or_else(|| "hex: n/a".to_string()),
      area.map(|area| format!("area: ({}, {})", area.get_x(), area.get_y()))
        .unwrap_or_else(|| "area: n/a".to_string()),
      format!("tile: ({}, {})", self.player_x, self.player_y),
    ]
  }

  pub fn update_visibility(&mut self) {
    if let Some(area) = self.view_manager.current_area_mut() {
      area.update_visibility(self.player_x, self.player_y);
      if let Some(label) = area.discover_visible_pois().into_iter().next() {
        self.show_popup(format!("Discovered: {label}"));
      }
    }
  }

  pub fn get_location_tile(&self, x: i32, y: i32) -> Option<Tile> {
    let area = self.view_manager.current_area()?;
    let terrain = area.get_visible_tile(x, y, &self.render_config)?;
    if area.is_visible(x, y) && self.enemy_at(x, y).is_some() {
      return Some(Tile::new('g', Color::Red));
    }
    Some(terrain)
  }

  fn current_location_id(&self) -> Option<LocationId> {
    let entity = self.view_manager.current_entity()?;
    self.entity_manager
      .get_component::<Location, _>(entity)
      .ok()?
      .get()
      .get_id()
      .ok()
  }

  fn enemy_at(&self, x:i32, y:i32) -> Option<controller::EnemyAtTile> {
    controller::enemy_at(&self.entity_manager, self.current_location_id()?, x, y)
  }

  pub async fn open_monster_picker(&mut self) -> Result<()> {
    if self.view_manager.is_in_world() {
      self.show_popup("Enter a local area before spawning an enemy.");
      return Ok(())
    }
    self.combat.open_picker().await
  }

  pub async fn spawn_selected_monster(&mut self) -> Result<()> {
    let Some(choice) = self.combat.selected_monster() else{return Ok(())};
    self.combat.close_picker();
    self.spawn_monster(choice).await
  }

  async fn spawn_monster(&mut self, choice: MonsterChoice) -> Result<()> {
    let location_id = self.current_location_id()
      .ok_or_else(|| anyhow!("current area has no location"))?;
    let position = {
      let area = self.view_manager.current_area()
        .ok_or_else(|| anyhow!("current area is unavailable"))?;
      let mut found = None;
      for radius in 2_i32..=8 {
        for dy in -radius..=radius {
          for dx in -radius..=radius {
            if dx.abs() != radius && dy.abs() != radius { continue }
            let candidate = (self.player_x + dx, self.player_y + dy);
            if area.is_walkable(candidate.0, candidate.1)
              && self.enemy_at(candidate.0, candidate.1).is_none()
            {
              found = Some(candidate);
              break
            }
          }
          if found.is_some() { break }
        }
        if found.is_some() { break }
      }
      found.ok_or_else(|| anyhow!("no nearby walkable tile can hold the monster"))?
    };
    self.entity_manager.create(MonsterCharacterSeed {
      name: choice.name.clone(),
      monster_type_key: choice.key,
      base_stats: BaseStatsData::default(),
      position: Some(CharacterPositionSeed {
        location_id,
        x: position.0,
        y: position.1,
      }),
    }).await?;
    self.combat.log(format!("{} enters the arena.",choice.name));
    Ok(())
  }

  pub fn player_hit_points(&self) -> Option<(i32,i32)> {
    self.combat.hit_points(&self.entity_manager)
  }

  pub async fn reset_combat_arena(&mut self) -> Result<()> {
    if !self.is_combat_arena(){
      self.show_popup("Arena reset is only available in the combat arena.");
      return Ok(())
    }
    let location_id=self.current_location_id()
      .ok_or_else(||anyhow!("combat arena has no location"))?;
    self.combat.reset_arena(&self.entity_manager,location_id).await?;
    Ok(())
  }

  pub async fn move_player(&mut self, dx: i32, dy: i32) -> Result<()> {
    let new_x = self.player_x + dx;
    let new_y = self.player_y + dy;

    if self.view_manager.is_in_world() {
      if self.map.get((new_x, new_y)).is_none() {
        self.generate_hex_at(new_x, new_y).await?;
      }
      self.player_x = new_x;
      self.player_y = new_y;
      self.camera.center_on(new_x, new_y);
      return Ok(())
    }

    let Some(area) = self.view_manager.current_area() else{return Ok(())};
    if area.in_bounds(new_x, new_y) {
      if self.player_hit_points().is_some_and(|(current,_)|current == 0) {
        self.combat.log("You are down. Press R to reset the arena.");
        return Ok(())
      }
      if let Some(enemy) = self.enemy_at(new_x, new_y) {
        if let Some(player) = self.combat.player() {
          let (strike,enemy_name) = controller::player_attack(
            self.combat.pool(),
            &self.entity_manager,
            player,
            enemy,
          ).await?;
          self.combat.record_player_attack(&enemy_name,&strike);
          self.advance_enemies().await?;
        }else{
          self.show_popup(format!("{} blocks your way.",enemy.name));
        }
        return Ok(())
      }
      let walkable = area.is_walkable(new_x, new_y);
      if walkable {
        if self.is_combat_arena() {
          self.combat.move_player(&self.entity_manager,(new_x,new_y)).await?;
        }
        self.player_x = new_x;
        self.player_y = new_y;
        self.camera.center_on(new_x, new_y);
        self.advance_enemies().await?;
      }
      return Ok(())
    }

    if self
      .view_manager
      .current_entity()
      .is_some_and(|entity| self.entity_manager.has::<Wilderness>(entity))
    {
      let outcome = self.view_manager.transition(
        TransitionIntent::CrossWildernessBoundary{target: (new_x, new_y)},
        (self.player_x, self.player_y),
        &self.entity_manager,
      )?;
      match self.resolve_transition(outcome).await? {
        TransitionOutcome::Ok(_) => {
          info!(coordinates = %self.coordinate_debug_lines().join(" | "), "crossed wilderness area boundary");
        }
        TransitionOutcome::WorldBoundary {
          world_dx,
          world_dy,
          target_area_x,
          target_area_y,
          crossing,
        } => {
          self.cross_world_boundary(
            world_dx,
            world_dy,
            target_area_x,
            target_area_y,
            crossing,
          ).await?;
        }
        _ => {}
      }
    }
    Ok(())
  }

  pub async fn wait_turn(&mut self) -> Result<()> {
    self.advance_enemies().await
  }

  async fn advance_enemies(&mut self) -> Result<()> {
    if !self.is_combat_arena(){return Ok(())}
    let Some(location_id)=self.current_location_id() else{return Ok(())};
    let Some(area)=self.view_manager.current_area() else{return Ok(())};
    let acted=self.combat.advance_enemies(
      &self.entity_manager,area,location_id,(self.player_x,self.player_y),
    ).await?;
    if acted{self.cancel_navigation();}
    Ok(())
  }

  pub async fn ensure_starting_hex(&mut self) -> Result<()> {
    if self.map.get((self.player_x, self.player_y)).is_none() {
      self.generate_hex_at(self.player_x, self.player_y).await?;
    }
    Ok(())
  }

  pub async fn enter_combat_arena(&mut self) -> Result<()> {
    let hex = self.map.get((0, 0))
      .ok_or_else(|| anyhow!("starting world container is unavailable"))?;
    let parent_location_id = self.entity_manager
      .get_component::<Location, _>(hex)?
      .get().get_id()?;
    let arena = match self.entity_manager
      .find_wilderness_area(parent_location_id, COMBAT_ARENA_KEY)
    {
      Some(entity) => entity,
      None => self.entity_manager.create(WildernessAreaSeed {
        area_key: COMBAT_ARENA_KEY.to_string(),
        x: 10_000,
        y: 10_000,
        width: COMBAT_ARENA_WIDTH,
        length: COMBAT_ARENA_HEIGHT,
        parent_location_id: Some(parent_location_id),
      }).await?,
    };
    let outcome = self.view_manager.transition(
      TransitionIntent::Enter(arena),
      (self.player_x, self.player_y),
      &self.entity_manager,
    )?;
    self.resolve_transition(outcome).await?;
    let location_id = self.current_location_id()
      .ok_or_else(|| anyhow!("combat arena has no location"))?;
    let position = self.combat.enter_arena(
      &self.entity_manager,
      location_id,
      (self.player_x,self.player_y),
    ).await?;
    self.player_x = position.0;
    self.player_y = position.1;
    self.camera.center_on(position.0,position.1);
    Ok(())
  }

  pub async fn generate_hex_at(&mut self, x: i32, y: i32) -> Result<()> {
    info!(x, y, "generating hex");
    let prev = self
      .map
      .get((x - 1, y))
      .and_then(|entity| self.entity_manager.get_component::<Hex, _>(entity).ok());

    let entity = self
      .entity_manager
      .create(HexSeed{x, y, prev})
      .await?;

    self.map.insert((x, y), entity);
    if let Ok(hex) = self.entity_manager.get_component::<Hex, _>(entity) {
      let tile = Tile::from_terrain_type(&hex.get());
      let mut world = self
        .entity_manager
        .world
        .lock()
        .map_err(|_| anyhow!("ECS world lock poisoned"))?;
      world.insert_one(entity, tile)?;
    }

    Ok(())
  }

  async fn cross_world_boundary(
    &mut self,
    world_dx: i32,
    world_dy: i32,
    target_area_x: i32,
    target_area_y: i32,
    crossing: WildernessCrossing,
  ) -> Result<()> {
    let (hex_x, hex_y) = self.current_hex_coordinates()?;
    let target_hex = (hex_x + world_dx, hex_y + world_dy);
    if self.map.get(target_hex).is_none() {
      self.generate_hex_at(target_hex.0, target_hex.1).await?;
    }
    let hex_entity = self
      .map
      .get(target_hex)
      .ok_or_else(|| anyhow!("unable to load destination world container"))?;
    let outcome = self.view_manager.transition(
      TransitionIntent::EnterWildernessArea {
        parent_entity: hex_entity,
        area_x: target_area_x,
        area_y: target_area_y,
        crossing,
      },
      (self.player_x, self.player_y),
      &self.entity_manager,
    )?;
    match self.resolve_transition(outcome).await? {
      TransitionOutcome::Ok(_) => {
        info!(coordinates = %self.coordinate_debug_lines().join(" | "), "crossed world-container boundary");
      }
      other => return Err(anyhow!("unexpected world-boundary transition: {other:?}")),
    }
    Ok(())
  }

  fn current_hex_coordinates(&self) -> Result<(i32, i32)> {
    if self.view_manager.is_in_world() {
      return Ok((self.player_x, self.player_y))
    }
    let entity = self.view_manager.current_entity().ok_or_else(|| anyhow!("no current area"))?;
    let parent = self.entity_manager.get_component::<Location, _>(entity)?
      .get().get_parent_location_id()?.ok_or_else(|| anyhow!("current area has no world-container parent"))?;
    let hex = self.entity_manager.find_entity_by_location_id::<Hex>(parent)
      .ok_or_else(|| anyhow!("unable to find current world container"))?;
    let spatial = self.entity_manager.get_component::<Spatial, _>(hex)?.get();
    Ok((spatial.get_x(), spatial.get_y()))
  }

  pub async fn generate_dungeon(&mut self) -> Result<()> {
    let dungeon = self
      .entity_manager
      .create(DungeonSeed{
        seed: 0,
        depth_levels: 1,
        x: 0,
        y: 0,
        parent_location_id: None,
      })
      .await?;

    let outcome = self.view_manager.transition(
      TransitionIntent::Enter(dungeon),
      (self.player_x, self.player_y),
      &self.entity_manager,
    )?;
    self.resolve_transition(outcome).await?;
    Ok(())
  }

  pub fn get_current_hex(&self) -> Result<Hex> {
    let entity = self
      .entity_manager
      .find_entity_at::<Hex>(self.player_x, self.player_y)
      .ok_or_else(|| anyhow!("unable to find current hex at player location"))?;
    self.entity_manager.get_component::<Hex, _>(entity)
  }

  pub fn rebuild_map(&mut self) {
    self.map.clear();
    let _ = self
      .entity_manager
      .for_each::<(&Hex, &Spatial), _>(|entity, (_hex, spatial)| {
        let spatial = spatial.get();
        self.map.insert((spatial.get_x(), spatial.get_y()), entity);
      });
  }

  pub fn attach_tiles(&mut self) {
    let mut tiles = Vec::new();
    let _ = self.entity_manager.for_each::<&Hex, _>(|entity, hex| {
      tiles.push((entity, Tile::from_terrain_type(&hex.get())));
    });

    if let Ok(mut world) = self.entity_manager.world.lock() {
      for (entity, tile) in tiles {
        let _ = world.insert_one(entity, tile);
      }
    }
  }

  pub async fn ascend(&mut self) -> Result<()> {
    let outcome = self.view_manager.transition(
      TransitionIntent::Ascend,
      (self.player_x, self.player_y),
      &self.entity_manager,
    )?;
    match self.resolve_transition(outcome).await? {
      TransitionOutcome::NotAtExit => self.navigate_to_stairs_up(),
      TransitionOutcome::AtWorldLevel => self.show_popup("Already at world level"),
      TransitionOutcome::Unsupported => self.show_popup("Cannot ascend from here"),
      _ => {}
    }
    Ok(())
  }

  pub async fn descend(&mut self) -> Result<()> {
    let outcome = if self.view_manager.is_in_world() {
      let hex = self
        .map
        .get((self.player_x, self.player_y))
        .ok_or_else(|| anyhow!("no hex found at player position"))?;
      self.view_manager.transition(
        TransitionIntent::Enter(hex),
        (self.player_x, self.player_y),
        &self.entity_manager,
      )?
    } else {
      self.view_manager.transition(
        TransitionIntent::Descend,
        (self.player_x, self.player_y),
        &self.entity_manager,
      )?
    };

    match self.resolve_transition(outcome).await? {
      TransitionOutcome::NotAtExit => self.navigate_to_stairs_down(),
      TransitionOutcome::NoEntry => self.show_popup("Nothing lies below this level"),
      TransitionOutcome::Unsupported => self.show_popup("Cannot descend from here"),
      _ => {}
    }
    Ok(())
  }

  async fn resolve_transition(
    &mut self,
    mut outcome: TransitionOutcome,
  ) -> Result<TransitionOutcome> {
    loop {
      match outcome {
        TransitionOutcome::NeedsAsync(work) => {
          outcome = self
            .view_manager
            .execute_async(work, &self.entity_manager)
            .await?;
        }
        TransitionOutcome::Ok(transition) => {
          self.apply_transition(&transition);
          return Ok(TransitionOutcome::Ok(transition));
        }
        other => return Ok(other),
      }
    }
  }

  fn apply_transition(&mut self, transition: &Transition) {
    self.cancel_navigation();
    self.player_x = transition.player_pos.0;
    self.player_y = transition.player_pos.1;
    self.camera.center_on(self.player_x, self.player_y);

    if transition.clear_fov {
      if let Some(area) = self.view_manager.current_area_mut() {
        area.clear_visibility();
      }
    }
    self.update_visibility();
  }

  pub fn dismiss_popup(&mut self) {
    self.popup_message = None;
  }

  pub fn is_looking(&self)->bool{self.look_mode.is_some()}
  pub fn look_cursor(&self)->Option<(i32,i32)>{self.look_mode.map(LookMode::cursor)}

  pub fn toggle_look(&mut self){
    if self.look_mode.is_some(){
      self.close_look();
      return
    }
    let Some(area)=self.view_manager.current_area() else{
      self.show_popup("Look mode is available inside local areas.");
      return
    };
    if area.in_bounds(self.player_x,self.player_y){
      self.popup_message=None;
      self.look_mode=Some(LookMode::new((self.player_x,self.player_y)));
    }
  }

  pub fn move_look_cursor(&mut self,dx:i32,dy:i32){
    let Some(area)=self.view_manager.current_area() else{return};
    if let Some(mode)=self.look_mode.as_mut(){
      mode.move_by(dx,dy,(area.width,area.height));
      let cursor=mode.cursor();
      self.camera.center_on(cursor.0,cursor.1);
    }
  }

  pub fn close_look(&mut self){
    self.look_mode=None;
    self.camera.center_on(self.player_x,self.player_y);
  }

  pub fn inspection_lines(&self)->Vec<String>{
    let Some(pos)=self.look_cursor() else{return Vec::new()};
    let Some(area)=self.view_manager.current_area() else{return Vec::new()};
    if !area.is_visible(pos.0,pos.1){
      return vec![format!("tile: ({}, {})",pos.0,pos.1),"Unseen".to_string()]
    }
    let actor=if pos==(self.player_x,self.player_y){
      Some("You".to_string())
    }else{
      self.enemy_at(pos.0,pos.1).map(|enemy|enemy.name)
    };
    resolver::inspect(area,pos,actor.as_deref()).lines()
  }

  pub fn show_popup(&mut self, message: impl Into<String>) {
    self.popup_message = Some(message.into());
  }

  pub fn is_auto_navigating(&self) -> bool {
    self.navigator.is_navigating()
  }

  pub fn cancel_navigation(&mut self) {
    self.navigator.cancel();
  }

  pub async fn step_navigation(&mut self) -> Result<bool> {
    if let Some((next_x, next_y)) = self.navigator.next_step() {
      let dx = next_x - self.player_x;
      let dy = next_y - self.player_y;
      self.move_player(dx, dy).await?;
      self.update_visibility();

      if self.navigator.is_exploring() && self.navigator.queue_is_empty() {
        self.find_next_exploration_target();
      }
      Ok(true)
    } else {
      Ok(false)
    }
  }

  pub fn start_exploring(&mut self) {
    if !self
      .view_manager
      .current_area()
      .is_some_and(|area| area.has_fov())
    {
      self.show_popup("Auto-explore is only available in areas with limited visibility");
      return;
    }

    self.navigator.start_exploring();
    self.find_next_exploration_target();
  }

  fn find_next_exploration_target(&mut self) {
    let start = (self.player_x, self.player_y);
    let result = match self.view_manager.current_area() {
      Some(area) => self.navigator.find_exploration_target(start, area),
      None => return,
    };

    if let Err(error) = result {
      self.show_popup(match error.to_string().as_str() {
        "area fully explored" => "Nowhere left to explore".to_string(),
        _ => format!("Auto-explore stopped: {error}"),
      });
    }
  }

  pub fn navigate_to_stairs_up(&mut self) {
    self.navigate_to_stairs(true);
  }

  pub fn navigate_to_stairs_down(&mut self) {
    self.navigate_to_stairs(false);
  }

  fn navigate_to_stairs(&mut self, going_up: bool) {
    let start = (self.player_x, self.player_y);
    let Some(area) = self.view_manager.current_area() else {
      self.show_popup("Not inside a location");
      return;
    };

    let already_there = if going_up {
      area.is_stairs_up(start.0, start.1)
    } else {
      area.is_stairs_down(start.0, start.1)
    };
    if already_there {
      self.show_popup(if going_up {
        "Already on stairs up - press `<` to ascend"
      } else {
        "Already on stairs down - press `>` to descend"
      });
      return;
    }

    let goals = if going_up {
      area.find_seen_stairs_up()
    } else {
      area.find_seen_stairs_down()
    };

    if let Err(error) = self.navigator.navigate_to_nearest(start, goals, area) {
      self.show_popup(format!("Cannot reach stairs: {error}"));
    }
  }
}
