use anyhow::{Result, anyhow};
use dagr_lib::components::world::{hex::Hex, spatial::Spatial};
use dagr_lib::core::registry::EntityKind;
use dagr_lib::ems::{component::Component, entity_manager::EntityManager};
use serde_json::json;
use tracing::info;

use crate::camera::Camera;
use crate::navigation::Navigator;
use crate::renderer::{RenderConfig, Tile};
use crate::views::{Transition, TransitionIntent, TransitionOutcome, ViewManager};
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
  pub popup_message: Option<String>,
}

impl GameState {
  pub fn new(entity_manager: EntityManager, view_w: u16, view_h: u16) -> Self {
    let mut state = Self {
      entity_manager,
      map: WorldMap::new(),
      camera: Camera::new(view_w, view_h),
      player_x: 0,
      player_y: 0,
      view_manager: ViewManager::new(),
      render_config: RenderConfig::default(),
      navigator: Navigator::new(),
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
    } else {
      "Wilderness".to_string()
    }
  }

  pub fn update_visibility(&mut self) {
    if let Some(area) = self.view_manager.current_area_mut() {
      area.update_visibility(self.player_x, self.player_y);
    }
  }

  pub fn get_location_tile(&self, x: i32, y: i32) -> Option<Tile> {
    self
      .view_manager
      .current_area()
      .and_then(|area| area.get_visible_tile(x, y, &self.render_config))
  }

  pub async fn move_player(&mut self, dx: i32, dy: i32) -> Result<()> {
    let new_x = self.player_x + dx;
    let new_y = self.player_y + dy;

    let can_move = if self.view_manager.is_in_world() {
      if self.map.get((new_x, new_y)).is_none() {
        self.generate_hex_at(new_x, new_y).await?;
      }
      true
    } else {
      self
        .view_manager
        .current_area()
        .is_some_and(|area| area.is_walkable(new_x, new_y))
    };

    if can_move {
      self.player_x = new_x;
      self.player_y = new_y;
      self.camera.center_on(new_x, new_y);
    }

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
      .create_entity(
        EntityKind::Hex,
        json!({
          "x": x,
          "y": y,
          "prev": prev,
        }),
      )
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

  pub async fn generate_dungeon(&mut self) -> Result<()> {
    let dungeon = self
      .entity_manager
      .create_entity(
        EntityKind::Dungeon,
        json!({
          "seed": 0,
          "depth_levels": 1,
          "x": 0,
          "y": 0,
        }),
      )
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
