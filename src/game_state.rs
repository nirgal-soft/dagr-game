use std::collections::HashSet;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use dagr_lib::{
    Engine, SpawnMonster, TravelIntent, TravelRequest, TravelStatus,
    characters::{CharacterId, CreateMonster},
    world::{
        Coordinate, Dimensions, GenerateHex, GenerateWilderness, HexProfile, HexRegion, HexView,
        LocationId, WildernessKey,
    },
};
use tracing::info;

use crate::camera::Camera;
use crate::combat::{picker::MonsterChoice, session::CombatSession};
use crate::debug_console::session::DebugSession;
use crate::generators::arena::{
    ArenaGenerator, COMBAT_ARENA_HEIGHT, COMBAT_ARENA_KEY, COMBAT_ARENA_WIDTH,
};
use crate::inspection::{mode::LookMode, resolver};
use crate::navigation::Navigator;
use crate::renderer::{RenderConfig, Tile};
use crate::views::ViewManager;
use crate::wilderness_layout::WildernessLayout;
use crate::world_map::WorldMap;

mod projection;

pub struct GameState {
    engine: Arc<Engine>,
    active_player: CharacterId,
    pub map: WorldMap,
    pub camera: Camera,
    pub player_x: i32,
    pub player_y: i32,
    pub view_manager: ViewManager,
    pub render_config: RenderConfig,
    navigator: Navigator,
    pub combat: CombatSession,
    pub debug: DebugSession,
    look_mode: Option<LookMode>,
    current_hex: Option<(i32, i32)>,
    current_area: Option<(i32, i32)>,
    current_wilderness_key: Option<String>,
    dungeon_offset: (i32, i32),
    pub popup_message: Option<String>,
}

impl GameState {
    pub async fn new(
        engine: Arc<Engine>,
        active_player: CharacterId,
        view_w: u16,
        view_h: u16,
        wilderness_layout: WildernessLayout,
    ) -> Result<Self> {
        let placement = engine
            .placement(active_player)
            .await?
            .ok_or_else(|| anyhow!("active character {active_player} has no placement"))?;
        let mut state = Self {
            combat: CombatSession::new(engine.clone()),
            engine,
            active_player,
            map: WorldMap::new(),
            camera: Camera::new(view_w, view_h),
            player_x: placement.x,
            player_y: placement.y,
            view_manager: ViewManager::new(wilderness_layout),
            render_config: RenderConfig::default(),
            navigator: Navigator::new(),
            debug: DebugSession::default(),
            look_mode: None,
            current_hex: None,
            current_area: None,
            current_wilderness_key: None,
            dungeon_offset: (0, 0),
            popup_message: None,
        };
        state.rebuild_map().await?;
        state.restore_view(active_player).await?;
        Ok(state)
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
        self.current_wilderness_key.as_deref() == Some(COMBAT_ARENA_KEY)
    }

    pub fn coordinate_debug_lines(&self) -> Vec<String> {
        if self.view_manager.is_in_world() {
            return vec![
                format!("hex: ({}, {})", self.player_x, self.player_y),
                "area: world map".to_string(),
                format!("tile: ({}, {})", self.player_x, self.player_y),
            ];
        }
        vec![
            self.current_hex
                .map(|(x, y)| format!("hex: ({x}, {y})"))
                .unwrap_or_else(|| "hex: n/a".to_string()),
            self.current_area
                .map(|(x, y)| format!("area: ({x}, {y})"))
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

    pub fn world_tile(&self, x: i32, y: i32) -> Option<Tile> {
        self.map
            .get((x, y))
            .map(|hex| Tile::from_hex_profile(&hex.profile))
    }

    pub fn get_location_tile(&self, x: i32, y: i32) -> Option<Tile> {
        self.view_manager
            .current_area()?
            .get_visible_tile(x, y, &self.render_config)
    }

    pub fn visible_enemy_positions(&self) -> HashSet<(i32, i32)> {
        let Some(area) = self.view_manager.current_area() else {
            return HashSet::new();
        };
        self.combat
            .enemy_positions()
            .filter(|(x, y)| area.is_visible(*x, *y))
            .collect()
    }

    fn current_location_id(&self) -> Option<LocationId> {
        self.view_manager.current_location()
    }

    pub async fn open_monster_picker(&mut self) -> Result<()> {
        if self.view_manager.is_in_world() {
            self.show_popup("Enter a local area before spawning an enemy.");
            return Ok(());
        }
        self.combat.open_picker().await
    }

    pub async fn spawn_selected_monster(&mut self) -> Result<()> {
        let Some(choice) = self.combat.selected_monster() else {
            return Ok(());
        };
        self.combat.close_picker();
        self.spawn_monster(choice).await
    }

    async fn spawn_monster(&mut self, choice: MonsterChoice) -> Result<()> {
        let location = self
            .current_location_id()
            .ok_or_else(|| anyhow!("current area has no location"))?;
        let position = {
            let area = self
                .view_manager
                .current_area()
                .ok_or_else(|| anyhow!("current area is unavailable"))?;
            let mut found = None;
            for radius in 2_i32..=8 {
                for dy in -radius..=radius {
                    for dx in -radius..=radius {
                        if dx.abs() != radius && dy.abs() != radius {
                            continue;
                        }
                        let candidate = (self.player_x + dx, self.player_y + dy);
                        if area.is_walkable(candidate.0, candidate.1)
                            && self.combat.enemy_at(candidate.0, candidate.1).is_none()
                        {
                            found = Some(candidate);
                            break;
                        }
                    }
                    if found.is_some() {
                        break;
                    }
                }
                if found.is_some() {
                    break;
                }
            }
            found.ok_or_else(|| anyhow!("no nearby walkable tile can hold the monster"))?
        };
        self.engine
            .spawn_monster(SpawnMonster {
                character: CreateMonster {
                    name: choice.name.clone(),
                    definition: choice.key,
                    seed: (u64::from(position.0.unsigned_abs()) << 32)
                        | u64::from(position.1.unsigned_abs()),
                    notable: false,
                },
                location,
                x: position.0,
                y: position.1,
            })
            .await?;
        self.combat.refresh_enemies(location).await?;
        self.combat
            .log(format!("{} enters the arena.", choice.name));
        Ok(())
    }

    pub fn player_hit_points(&self) -> Option<(i32, i32)> {
        self.combat.hit_points()
    }

    pub async fn reset_combat_arena(&mut self) -> Result<()> {
        if !self.is_combat_arena() {
            self.show_popup("Arena reset is only available in the combat arena.");
            return Ok(());
        }
        let location = self
            .current_location_id()
            .ok_or_else(|| anyhow!("combat arena has no location"))?;
        self.combat.reset_arena(location).await
    }

    pub async fn move_player(&mut self, dx: i32, dy: i32) -> Result<()> {
        if self.view_manager.is_in_world() {
            let outcome = self
                .engine
                .travel(TravelRequest {
                    actor: self.active_player,
                    intent: TravelIntent::WorldStep { dx, dy },
                })
                .await?;
            self.apply_travel(outcome).await?;
            return Ok(());
        }

        let new_x = self.player_x + dx;
        let new_y = self.player_y + dy;
        let Some(area) = self.view_manager.current_area() else {
            return Ok(());
        };
        if area.in_bounds(new_x, new_y) {
            if self
                .player_hit_points()
                .is_some_and(|(current, _)| current == 0)
            {
                self.combat.log("You are down. Press R to reset the arena.");
                return Ok(());
            }
            if let Some(enemy) = self.combat.enemy_at(new_x, new_y).cloned() {
                if self.combat.player().is_some() {
                    self.combat.player_attack(enemy.id).await?;
                    self.advance_enemies().await?;
                } else {
                    self.show_popup(format!("{} blocks your way.", enemy.name));
                }
                return Ok(());
            }
            if !area.is_walkable(new_x, new_y) {
                return Ok(());
            }
            let actor = self.current_actor();
            let outcome = self
                .engine
                .travel(TravelRequest {
                    actor,
                    intent: TravelIntent::LocalMove { dx, dy },
                })
                .await?;
            self.apply_travel(outcome).await?;
            self.advance_enemies().await?;
            return Ok(());
        }

        if self.current_wilderness_key.is_some() {
            let width = area.width;
            let height = area.height;
            let world_dx = if new_x < 0 {
                -1
            } else if new_x >= width {
                1
            } else {
                0
            };
            let world_dy = if new_y < 0 {
                -1
            } else if new_y >= height {
                1
            } else {
                0
            };
            let outcome = self
                .engine
                .travel(TravelRequest {
                    actor: self.current_actor(),
                    intent: TravelIntent::CrossWilderness {
                        dx: world_dx,
                        dy: world_dy,
                    },
                })
                .await?;
            self.apply_travel(outcome).await?;
            info!(coordinates = %self.coordinate_debug_lines().join(" | "), "crossed wilderness boundary");
        }
        Ok(())
    }

    pub async fn wait_turn(&mut self) -> Result<()> {
        self.advance_enemies().await
    }

    async fn advance_enemies(&mut self) -> Result<()> {
        if !self.is_combat_arena() {
            return Ok(());
        }
        let Some(location) = self.current_location_id() else {
            return Ok(());
        };
        if self.combat.advance_enemies(location).await? {
            self.cancel_navigation();
        }
        Ok(())
    }

    pub async fn ensure_starting_hex(&mut self) -> Result<()> {
        if self.map.get((0, 0)).is_none() {
            self.generate_hex_at(0, 0).await?;
        }
        Ok(())
    }

    pub async fn enter_combat_arena(&mut self) -> Result<()> {
        let origin = self
            .map
            .get((0, 0))
            .ok_or_else(|| anyhow!("starting world container is unavailable"))?;
        let parent = origin.location_id;
        let key = WildernessKey::within(parent, COMBAT_ARENA_KEY)?;
        let arena = match self.engine.world().wilderness_area(key.clone()).await? {
            Some(arena) => arena,
            None => {
                self.engine
                    .world()
                    .generate_wilderness(GenerateWilderness {
                        area_key: key,
                        coordinate: Coordinate::new(10_000, 10_000, None),
                        dimensions: Dimensions::new(COMBAT_ARENA_WIDTH, COMBAT_ARENA_HEIGHT, None)?,
                        parent_location_id: Some(parent),
                    })
                    .await?
                    .wilderness
            }
        };
        let center = (COMBAT_ARENA_WIDTH / 2, COMBAT_ARENA_HEIGHT / 2);
        self.combat.enter_arena(arena.location_id, center).await?;
        self.view_manager.show_area(
            arena.location_id,
            None,
            ArenaGenerator::generate(COMBAT_ARENA_WIDTH, COMBAT_ARENA_HEIGHT),
        );
        self.current_wilderness_key = Some(COMBAT_ARENA_KEY.to_string());
        self.current_area = Some((arena.spatial.coordinate.x, arena.spatial.coordinate.y));
        self.current_hex = Some((0, 0));
        self.player_x = center.0;
        self.player_y = center.1;
        self.camera.center_on(center.0, center.1);
        Ok(())
    }

    pub async fn place_debug_hex(&mut self, x: i32, y: i32, profile: HexProfile) -> Result<bool> {
        if self.map.get((x, y)).is_some() {
            return Ok(false);
        }
        match self
            .engine
            .world()
            .generate_hex(GenerateHex {
                coordinate: dagr_lib::world::HexCoordinate::new(x, y),
                previous: None,
                profile: Some(profile),
            })
            .await
        {
            Ok(outcome) => {
                self.map.insert(outcome.hex);
                Ok(true)
            }
            Err(dagr_lib::world::Error::Conflict { .. }) => Ok(false),
            Err(error) => Err(error.into()),
        }
    }

    pub async fn generate_hex_at(&mut self, x: i32, y: i32) -> Result<()> {
        if self.map.get((x, y)).is_some() {
            return Ok(());
        }
        info!(x, y, "generating hex");
        let previous = self.map.get((x - 1, y)).map(|hex| hex.profile.clone());
        match self
            .engine
            .world()
            .generate_hex(GenerateHex::generated(x, y, previous))
            .await
        {
            Ok(outcome) => self.map.insert(outcome.hex),
            Err(dagr_lib::world::Error::Conflict { .. }) => {
                if let Some(hex) = self
                    .engine
                    .world()
                    .hex_at(dagr_lib::world::HexCoordinate::new(x, y))
                    .await?
                {
                    self.map.insert(hex);
                }
            }
            Err(error) => return Err(error.into()),
        }
        Ok(())
    }

    pub async fn generate_dungeon(&mut self) -> Result<()> {
        let parent = if self.is_in_world() {
            self.map
                .get((self.player_x, self.player_y))
                .map(|hex| hex.location_id)
        } else {
            self.current_hex_location()
        };
        let dungeon = self.ensure_dungeon(parent).await?;
        self.enter_dungeon(dungeon).await
    }

    pub fn get_current_hex(&self) -> Result<HexView> {
        let coordinates = self.current_hex.unwrap_or((self.player_x, self.player_y));
        self.map
            .get(coordinates)
            .cloned()
            .ok_or_else(|| anyhow!("unable to find current hex at player location"))
    }

    pub async fn rebuild_map(&mut self) -> Result<()> {
        self.map.clear();
        let region = HexRegion::new(
            dagr_lib::world::HexCoordinate::new(-32, -32),
            dagr_lib::world::HexCoordinate::new(31, 31),
        )?;
        for hex in self.engine.world().hexes(region).await? {
            self.map.insert(hex);
        }
        Ok(())
    }

    pub async fn ascend(&mut self) -> Result<()> {
        if self.view_manager.is_in_world() {
            self.show_popup("Already at world level");
            return Ok(());
        }
        let intent = if self.view_manager.current_level().is_some() {
            TravelIntent::DungeonAscend
        } else {
            TravelIntent::Exit
        };
        let outcome = self
            .engine
            .travel(TravelRequest {
                actor: self.current_actor(),
                intent,
            })
            .await?;
        let status = outcome.status;
        self.apply_travel(outcome).await?;
        match status {
            TravelStatus::NoEntry => self.navigate_to_stairs_up(),
            TravelStatus::Unsupported => self.show_popup("Cannot ascend from here"),
            _ => {}
        }
        Ok(())
    }

    pub async fn descend(&mut self) -> Result<()> {
        if self.view_manager.is_in_world() {
            let hex = self
                .map
                .get((self.player_x, self.player_y))
                .cloned()
                .ok_or_else(|| anyhow!("no hex found at player position"))?;
            let (width, height) = self.view_manager.wilderness_dimensions();
            let key = WildernessKey::within(hex.location_id, "origin")?;
            let wilderness = match self.engine.world().wilderness_area(key.clone()).await? {
                Some(view) => view,
                None => {
                    self.engine
                        .world()
                        .generate_wilderness(GenerateWilderness {
                            area_key: key,
                            coordinate: Coordinate::new(0, 0, None),
                            dimensions: Dimensions::new(width, height, None)?,
                            parent_location_id: Some(hex.location_id),
                        })
                        .await?
                        .wilderness
                }
            };
            let outcome = self
                .engine
                .travel(TravelRequest {
                    actor: self.active_player,
                    intent: TravelIntent::Enter {
                        location: wilderness.location_id,
                        x: width / 2,
                        y: height / 2,
                    },
                })
                .await?;
            self.apply_travel(outcome).await?;
            return Ok(());
        }

        if self.view_manager.current_level().is_some() {
            let outcome = self
                .engine
                .travel(TravelRequest {
                    actor: self.current_actor(),
                    intent: TravelIntent::DungeonDescend,
                })
                .await?;
            let status = outcome.status;
            self.apply_travel(outcome).await?;
            if status == TravelStatus::NoEntry {
                self.navigate_to_stairs_down();
            }
            return Ok(());
        }

        let Some(area) = self.view_manager.current_area() else {
            return Ok(());
        };
        if !area.is_stairs_down(self.player_x, self.player_y) {
            self.navigate_to_stairs_down();
            return Ok(());
        }
        let dungeon = self.ensure_dungeon(self.current_hex_location()).await?;
        self.enter_dungeon(dungeon).await
    }

    pub fn dismiss_popup(&mut self) {
        self.popup_message = None;
    }

    pub fn is_looking(&self) -> bool {
        self.look_mode.is_some()
    }

    pub fn look_cursor(&self) -> Option<(i32, i32)> {
        self.look_mode.map(LookMode::cursor)
    }

    pub fn toggle_look(&mut self) {
        if self.look_mode.is_some() {
            self.close_look();
            return;
        }
        let Some(area) = self.view_manager.current_area() else {
            self.show_popup("Look mode is available inside local areas.");
            return;
        };
        if area.in_bounds(self.player_x, self.player_y) {
            self.popup_message = None;
            self.look_mode = Some(LookMode::new((self.player_x, self.player_y)));
        }
    }

    pub fn move_look_cursor(&mut self, dx: i32, dy: i32) -> bool {
        let Some(area) = self.view_manager.current_area() else {
            return false;
        };
        let camera_before = (self.camera.x, self.camera.y);
        if let Some(mode) = self.look_mode.as_mut() {
            mode.move_by(dx, dy, (area.width, area.height));
            let cursor = mode.cursor();
            self.camera.pan_when_border_reached(cursor.0, cursor.1);
        }
        camera_before != (self.camera.x, self.camera.y)
    }

    pub fn close_look(&mut self) {
        self.look_mode = None;
        self.camera.center_on(self.player_x, self.player_y);
    }

    pub fn inspection_lines(&self) -> Vec<String> {
        let Some(pos) = self.look_cursor() else {
            return Vec::new();
        };
        let Some(area) = self.view_manager.current_area() else {
            return Vec::new();
        };
        if !area.is_visible(pos.0, pos.1) {
            return vec![
                format!("tile: ({}, {})", pos.0, pos.1),
                "Unseen".to_string(),
            ];
        }
        let actor = if pos == (self.player_x, self.player_y) {
            Some("You".to_string())
        } else {
            self.combat
                .enemy_at(pos.0, pos.1)
                .map(|enemy| enemy.name.clone())
        };
        resolver::inspect(area, pos, actor.as_deref()).lines()
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
