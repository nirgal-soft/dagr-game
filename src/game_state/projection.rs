//! Engine travel outcomes projected into client-owned views.

use anyhow::{Result, anyhow};
use dagr_lib::{
    TravelIntent, TravelOutcome, TravelRequest, TravelStatus,
    characters::CharacterId,
    world::{
        AreaView, Coordinate, DungeonView, GenerateDungeon, LocationId, LocationKind, LocationView,
        WildernessView,
    },
};

use super::GameState;
use crate::areas::{Feature, PoiKind, PointOfInterest};
use crate::generators::{
    arena::{ArenaGenerator, COMBAT_ARENA_HEIGHT, COMBAT_ARENA_KEY, COMBAT_ARENA_WIDTH},
    dungeon::DungeonGenerator,
    wilderness::WildernessGenerator,
};

impl GameState {
    pub(super) async fn restore_view(&mut self, actor: CharacterId) -> Result<()> {
        let placement = self
            .engine
            .placement(actor)
            .await?
            .ok_or_else(|| anyhow!("character {actor} has no placement"))?;
        let destination = self.engine.world().location(placement.location).await?;
        let area = match destination.facts().kind {
            LocationKind::Wilderness => Some(
                self.engine
                    .world()
                    .area(
                        destination.facts().id,
                        dagr_lib::world::AreaLevel::Wilderness,
                    )
                    .await?,
            ),
            LocationKind::DungeonRoom => {
                let parent = destination
                    .facts()
                    .parent
                    .ok_or_else(|| anyhow!("dungeon room has no dungeon parent"))?;
                let LocationView::Dungeon(dungeon) = self.engine.world().location(parent).await?
                else {
                    return Err(anyhow!("dungeon room parent is not a dungeon"));
                };
                let level = dungeon
                    .rooms
                    .iter()
                    .find(|room| room.location.id == placement.location)
                    .map(|room| room.level)
                    .ok_or_else(|| anyhow!("dungeon room is absent from its aggregate"))?;
                Some(
                    self.engine
                        .world()
                        .area(parent, dagr_lib::world::AreaLevel::Dungeon(level))
                        .await?,
                )
            }
            _ => None,
        };
        self.apply_travel(TravelOutcome {
            placement,
            destination,
            area,
            status: TravelStatus::Moved,
        })
        .await
    }

    pub(super) async fn apply_travel(&mut self, outcome: TravelOutcome) -> Result<()> {
        match &outcome.area {
            None => {
                let LocationView::Hex(hex) = &outcome.destination else {
                    return Err(anyhow!("world travel did not resolve to a hex"));
                };
                self.map.insert(hex.clone());
                self.view_manager.show_world();
                self.player_x = hex.coordinate.x;
                self.player_y = hex.coordinate.y;
                self.current_hex = Some((hex.coordinate.x, hex.coordinate.y));
                self.current_area = None;
                self.current_wilderness_key = None;
                self.dungeon_offset = (0, 0);
            }
            Some(AreaView::Wilderness(wilderness)) => {
                if !self.view_manager.cached_area(wilderness.location_id, None) {
                    let area = self.build_wilderness_area(wilderness).await?;
                    self.view_manager
                        .show_area(wilderness.location_id, None, area);
                }
                self.player_x = outcome.placement.x;
                self.player_y = outcome.placement.y;
                self.current_area = Some((
                    wilderness.spatial.coordinate.x,
                    wilderness.spatial.coordinate.y,
                ));
                self.current_wilderness_key = Some(wilderness.area_key.as_str().to_string());
                self.current_hex = self.hex_coordinates_for(wilderness.parent).await?;
                self.dungeon_offset = (0, 0);
            }
            Some(AreaView::DungeonLevel(level)) => {
                let LocationView::Dungeon(dungeon) = self
                    .engine
                    .world()
                    .location(level.dungeon_location_id)
                    .await?
                else {
                    return Err(anyhow!("dungeon area root is not a dungeon"));
                };
                if !self
                    .view_manager
                    .cached_area(level.dungeon_location_id, Some(level.level))
                {
                    let area = DungeonGenerator::new(0).generate(&dungeon, level.level)?;
                    self.view_manager
                        .show_area(level.dungeon_location_id, Some(level.level), area);
                }
                let offset = dungeon_level_offset(&dungeon, level.level)?;
                self.dungeon_offset = offset;
                self.player_x = outcome.placement.x - offset.0;
                self.player_y = outcome.placement.y - offset.1;
                self.current_area = None;
                self.current_wilderness_key = None;
                self.current_hex = self.hex_coordinates_for(dungeon.location.parent).await?;
            }
        }
        self.camera.center_on(self.player_x, self.player_y);
        self.cancel_navigation();
        self.update_visibility();
        match outcome.status {
            TravelStatus::Blocked => self.show_popup("That way is blocked."),
            TravelStatus::Edge => self.show_popup("You have reached the edge."),
            TravelStatus::Unsupported => self.show_popup("Cannot travel that way from here."),
            _ => {}
        }
        Ok(())
    }

    async fn build_wilderness_area(
        &self,
        wilderness: &WildernessView,
    ) -> Result<crate::areas::Area> {
        if wilderness.area_key.as_str() == COMBAT_ARENA_KEY {
            return Ok(ArenaGenerator::generate(
                COMBAT_ARENA_WIDTH,
                COMBAT_ARENA_HEIGHT,
            ));
        }
        let (width, height) = self.view_manager.wilderness_dimensions();
        let profile = match wilderness.parent {
            Some(parent) => match self.engine.world().location(parent).await? {
                LocationView::Hex(hex) => hex.profile,
                _ => self.get_current_hex()?.profile,
            },
            None => self.get_current_hex()?.profile,
        };
        let mut area = WildernessGenerator::for_hex(wilderness.generation_seed as u64, &profile)
            .generate(width, height)?;
        if let Some(position) = area.find_feature(Feature::LANDMARK) {
            let kind = PoiKind::from_content_key(&profile.point_of_interest);
            let mut poi = PointOfInterest::new(position, kind, wilderness.generation_seed as u64);
            if let (Some(parent), Some(location_kind)) =
                (wilderness.parent, kind.enterable_location_kind())
            {
                if let Some(child) = self
                    .engine
                    .world()
                    .children(parent)
                    .await?
                    .into_iter()
                    .find(|child| child.kind == location_kind)
                {
                    poi = poi.with_location(child.id);
                }
            }
            area.add_poi(poi);
        }
        Ok(area)
    }

    pub(super) async fn ensure_dungeon(&self, parent: Option<LocationId>) -> Result<DungeonView> {
        if let Some(parent) = parent {
            for child in self.engine.world().children(parent).await? {
                if child.kind == LocationKind::Dungeon {
                    if let LocationView::Dungeon(dungeon) =
                        self.engine.world().location(child.id).await?
                    {
                        return Ok(dungeon);
                    }
                }
            }
        }
        Ok(self
            .engine
            .world()
            .generate_dungeon(GenerateDungeon {
                seed: 0,
                depth_levels: 1,
                coordinate: Coordinate::new(0, 0, None),
                parent_location_id: parent,
            })
            .await?
            .dungeon)
    }

    pub(super) async fn enter_dungeon(&mut self, dungeon: DungeonView) -> Result<()> {
        let entrance = dungeon
            .rooms
            .iter()
            .find(|room| room.is_entrance && room.level == 1)
            .ok_or_else(|| anyhow!("dungeon has no level-one entrance"))?;
        let outcome = self
            .engine
            .travel(TravelRequest {
                actor: self.current_actor(),
                intent: TravelIntent::Enter {
                    location: dungeon.location_id,
                    x: entrance.spatial.coordinate.x,
                    y: entrance.spatial.coordinate.y,
                },
            })
            .await?;
        if outcome.status == TravelStatus::NoEntry {
            self.show_popup("Nothing lies below this level");
            return Ok(());
        }
        self.apply_travel(outcome).await
    }

    pub(super) fn current_actor(&self) -> CharacterId {
        if self.is_combat_arena() {
            self.combat.player().unwrap_or(self.active_player)
        } else {
            self.active_player
        }
    }

    pub(super) fn current_hex_location(&self) -> Option<LocationId> {
        self.current_hex
            .and_then(|coordinates| self.map.get(coordinates))
            .map(|hex| hex.location_id)
    }

    async fn hex_coordinates_for(
        &self,
        mut location: Option<LocationId>,
    ) -> Result<Option<(i32, i32)>> {
        while let Some(id) = location {
            let view = self.engine.world().location(id).await?;
            if let LocationView::Hex(hex) = view {
                return Ok(Some((hex.coordinate.x, hex.coordinate.y)));
            }
            location = view.facts().parent;
        }
        Ok(None)
    }
}

fn dungeon_level_offset(dungeon: &DungeonView, level: i32) -> Result<(i32, i32)> {
    let mut rooms = dungeon.rooms.iter().filter(|room| room.level == level);
    let first = rooms
        .next()
        .ok_or_else(|| anyhow!("dungeon level {level} has no rooms"))?;
    Ok(rooms.fold(
        (first.spatial.coordinate.x, first.spatial.coordinate.y),
        |(min_x, min_y), room| {
            (
                min_x.min(room.spatial.coordinate.x),
                min_y.min(room.spatial.coordinate.y),
            )
        },
    ))
}
