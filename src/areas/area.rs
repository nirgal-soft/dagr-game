use super::{Feature, Fixture, Ground, LocationConfig, PoiKind, PointOfInterest, Pos};
use crate::renderer::{RenderConfig, Tile};
use crate::visiblity::{Visibility, VisibilityMap};
use dagr_lib::components::world::location::LocationType;
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub struct Area {
  pub width: i32,
  pub height: i32,
  ground: Ground,
  features: HashMap<Pos, Feature>,
  fixtures: HashMap<Pos, Vec<Fixture>>,
  pois: HashMap<Pos, PointOfInterest>,
  seen: HashSet<Pos>,
  visibility: Option<VisibilityMap>,
  pub entrance: Option<Pos>,
  pub stairs_up: Option<Pos>,
  pub stairs_down: Option<Pos>,
  pub current_level: Option<i32>,
  pub max_level: Option<i32>,
}

impl Area {
  pub fn new(width: i32, height: i32, ground: Ground) -> Self {
    Self {
      width,
      height,
      ground,
      features: HashMap::new(),
      fixtures: HashMap::new(),
      pois: HashMap::new(),
      seen: HashSet::new(),
      visibility: None,
      entrance: None,
      stairs_up: None,
      stairs_down: None,
      current_level: None,
      max_level: None,
    }
  }

  pub fn for_location_type(location_type: LocationType, width: i32, height: i32) -> Self {
    let config = LocationConfig::for_type(location_type);
    let mut area = Self::new(width, height, config.ground);
    if config.has_fov {
      area.visibility = Some(VisibilityMap::new(8));
    }
    area
  }

  pub fn dungeon(width: i32, height: i32) -> Self {
    Self::for_location_type(LocationType::Dungeon, width, height)
  }

  pub fn wilderness(width: i32, height: i32) -> Self {
    Self::for_location_type(LocationType::Wilderness, width, height)
  }

  //---accessors and mutators---
  pub fn set_ground(&mut self, ground: Ground) {
    self.ground = ground;
  }
  pub fn set_current_level(&mut self, level: i32) {
    self.current_level = Some(level);
  }
  pub fn set_max_level(&mut self, level: i32) {
    self.max_level = Some(level);
  }
  pub fn set_feature(&mut self, x: i32, y: i32, feature: Feature) {
    self.features.insert((x, y), feature);
  }
  pub fn get_feature(&self, x: i32, y: i32) -> Option<&Feature> {
    self.features.get(&(x, y))
  }
  pub fn remove_feature(&mut self, x: i32, y: i32) {
    self.features.remove(&(x, y));
  }
  pub fn find_feature(&self, feature: Feature) -> Option<Pos> {
    self.features
      .iter()
      .find_map(|(pos, candidate)| (*candidate == feature).then_some(*pos))
  }
  pub fn add_fixture(&mut self, fixture: Fixture) {
    self.fixtures.entry(fixture.pos).or_default().push(fixture);
  }
  pub fn fixtures_at(&self, x: i32, y: i32) -> &[Fixture] {
    self.fixtures.get(&(x, y)).map(Vec::as_slice).unwrap_or(&[])
  }
  pub fn ground(&self) -> Ground {
    self.ground
  }

  //---convience mutators for common features---
  pub fn set_wall(&mut self, x: i32, y: i32) {
    self.set_feature(x, y, Feature::WALL);
  }
  pub fn set_tree(&mut self, x: i32, y: i32) {
    self.set_feature(x, y, Feature::TREE);
  }
  pub fn set_rock(&mut self, x: i32, y: i32) {
    self.set_feature(x, y, Feature::ROCK);
  }
  pub fn set_water(&mut self, x: i32, y: i32) {
    self.set_feature(x, y, Feature::WATER);
  }
  pub fn set_entrance(&mut self, x: i32, y: i32) {
    self.entrance = Some((x, y));
  }
  pub fn set_stairs_up(&mut self, x: i32, y: i32) {
    self.stairs_up = Some((x, y));
    self.set_feature(x, y, Feature::STAIRS_UP);
  }
  pub fn set_stairs_down(&mut self, x: i32, y: i32) {
    self.stairs_down = Some((x, y));
    self.set_feature(x, y, Feature::STAIRS_DOWN);
  }

  //---poi access---
  pub fn add_poi(&mut self, poi: PointOfInterest) {
    let pos = poi.pos;
    let feature = poi.kind.feature();
    if poi.kind.enterable_location_type() == Some(LocationType::Dungeon) {
      self.stairs_down = Some(pos);
    }
    self.set_feature(pos.0, pos.1, feature);
    self.pois.insert(pos, poi);
  }
  pub fn get_poi_at(&self, pos: Pos) -> Option<&PointOfInterest> {
    self.pois.get(&pos)
  }
  pub fn get_poi_at_mut(&mut self, pos: Pos) -> Option<&mut PointOfInterest> {
    self.pois.get_mut(&pos)
  }
  pub fn pois(&self) -> impl Iterator<Item = &PointOfInterest> {
    self.pois.values()
  }
  pub fn has_poi_at(&self, pos: Pos) -> bool {
    self.pois.contains_key(&pos)
  }
  pub fn discover_visible_pois(&mut self) -> Vec<String> {
    let visible = self
      .pois
      .iter()
      .filter_map(|(pos, poi)| (!poi.discovered && self.is_visible(pos.0, pos.1)).then_some(*pos))
      .collect::<Vec<_>>();
    let mut labels = Vec::new();
    for pos in visible {
      if let Some(poi) = self.pois.get_mut(&pos) {
        poi.discovered = true;
        labels.push(poi.label.clone());
      }
    }
    labels
  }
  pub fn find_poi_by_entity(&self, entity: hecs::Entity) -> Option<Pos> {
    self
      .pois
      .values()
      .find(|poi| poi.entity == Some(entity))
      .map(|poi| poi.pos)
  }

  //---spatial queries---
  pub fn in_bounds(&self, x: i32, y: i32) -> bool {
    x >= 0 && x < self.width && y >= 0 && y < self.height
  }

  pub fn is_walkable(&self, x: i32, y: i32) -> bool {
    if !self.in_bounds(x, y) {
      return false;
    }
    self.features.get(&(x, y)).is_none_or(|f| f.walkable)
  }

  pub fn is_opaque(&self, x: i32, y: i32) -> bool {
    if !self.in_bounds(x, y) {
      return true;
    }
    self.features.get(&(x, y)).is_some_and(|f| f.opaque)
  }

  pub fn is_stairs_up(&self, x: i32, y: i32) -> bool {
    self.features.get(&(x, y)).is_some_and(|f| f.is_stairs_up())
  }

  pub fn is_stairs_down(&self, x: i32, y: i32) -> bool {
    self
      .features
      .get(&(x, y))
      .is_some_and(|f| f.is_stairs_down())
  }

  //---visibility---
  pub fn has_fov(&self) -> bool {
    self.visibility.is_some()
  }

  pub fn update_visibility(&mut self, player_x: i32, player_y: i32) {
    let opaque_check: Vec<Vec<bool>> = (0..self.height)
      .map(|y| (0..self.width).map(|x| self.is_opaque(x, y)).collect())
      .collect();

    if let Some(ref mut vis) = self.visibility {
      vis.update(player_x, player_y, |x, y| {
        opaque_check
          .get(y as usize)
          .and_then(|row| row.get(x as usize))
          .copied()
          .unwrap_or(true)
      });
      let visible: Vec<_> = vis.visible_tiles().collect();
      self.seen.extend(visible);
    }
  }

  pub fn clear_visibility(&mut self) {
    if let Some(ref mut vis) = self.visibility {
      vis.clear();
    }
  }

  pub fn is_visible(&self, x: i32, y: i32) -> bool {
    match &self.visibility {
      Some(vis) => vis.is_visible(x, y),
      None => true,
    }
  }

  pub fn is_seen(&self, x: i32, y: i32) -> bool {
    match &self.visibility {
      Some(_) => self.seen.contains(&(x, y)),
      None => true,
    }
  }

  pub fn seen_tiles(&self) -> impl Iterator<Item = &Pos> {
    self.seen.iter()
  }

  pub fn seen_count(&self) -> usize {
    self.seen.len()
  }

  pub fn is_seen_and_walkable(&self, x: i32, y: i32) -> bool {
    self.is_seen(x, y) && self.is_walkable(x, y)
  }

  //---rendering---
  pub fn get_tile(&self, x: i32, y: i32) -> Tile {
    self.fixtures_at(x, y).last().map(|fixture| fixture.kind.tile())
      .or_else(|| self.features.get(&(x, y)).map(|feature| feature.tile))
      .unwrap_or(self.ground.tile)
  }

  pub fn get_visible_tile(&self, x: i32, y: i32, config: &RenderConfig) -> Option<Tile> {
    if !self.in_bounds(x, y) {
      return None;
    }

    if self.is_visible(x, y) {
      Some(self.get_tile(x, y))
    } else if self.is_seen(x, y) {
      Some(
        self
          .get_tile(x, y)
          .with_visibility(Visibility::Seen, config),
      )
    } else {
      None
    }
  }

  //--exploration---
  pub fn find_seen_stairs_up(&self) -> Vec<Pos> {
    self
      .stairs_up
      .into_iter()
      .filter(|p| self.is_seen(p.0, p.1))
      .collect()
  }

  pub fn find_seen_stairs_down(&self) -> Vec<Pos> {
    self
      .stairs_down
      .into_iter()
      .filter(|p| self.is_seen(p.0, p.1))
      .collect()
  }

  pub fn find_unseen_walkable(&self) -> Vec<Pos> {
    let mut result = Vec::new();
    for x in 0..self.width {
      for y in 0..self.height {
        if self.is_walkable(x, y) && !self.is_seen(x, y) {
          result.push((x, y));
        }
      }
    }

    result
  }

  pub fn find_exploration_frontiers(&self) -> Vec<Pos> {
    let mut frontiers = Vec::new();

    for &(x, y) in &self.seen {
      if !self.is_walkable(x, y) {
        continue;
      }

      let neighbors = [
        (x - 1, y - 1),
        (x, y - 1),
        (x + 1, y - 1),
        (x - 1, y),
        (x + 1, y),
        (x - 1, y + 1),
        (x, y + 1),
        (x + 1, y + 1),
      ];

      for (nx, ny) in neighbors {
        if self.in_bounds(nx, ny) && !self.seen.contains(&(nx, ny)) {
          frontiers.push((x, y));
          break;
        }
      }
    }

    frontiers
  }

  pub fn is_fully_explored(&self) -> bool {
    self.find_exploration_frontiers().is_empty()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn ground_is_walkable_and_transparent() {
    let area = Area::dungeon(10, 10);
    assert!(area.is_walkable(5, 5));
    assert!(!area.is_opaque(5, 5));
    assert!(!area.is_walkable(-1, 5));
    assert!(area.is_opaque(-1, 5));
  }

  #[test]
  fn wilderness_uses_local_field_of_view() {
    let mut area = Area::wilderness(30, 30);
    assert!(area.has_fov());
    assert!(!area.is_visible(15, 15));
    area.update_visibility(15, 15);
    assert!(area.is_visible(15, 15));
    assert!(area.is_visible(18, 15));
    assert!(!area.is_visible(29, 29));
  }

  #[test]
  fn trees_and_water_are_traversable_local_terrain() {
    let mut area = Area::wilderness(10, 10);
    area.set_tree(2, 2);
    area.set_water(3, 3);
    assert!(area.is_walkable(2, 2));
    assert!(area.is_opaque(2, 2));
    assert!(area.is_walkable(3, 3));
    assert!(!area.is_opaque(3, 3));
  }

  #[test]
  fn visible_pois_are_discovered_only_once() {
    let mut area = Area::wilderness(20, 20);
    area.add_poi(PointOfInterest::new((6, 5), PoiKind::Ruins, 42));
    assert!(area.discover_visible_pois().is_empty());
    area.update_visibility(5, 5);
    assert_eq!(area.discover_visible_pois(), vec!["Ancient ruins"]);
    assert!(area.discover_visible_pois().is_empty());
    assert_eq!(area.get_tile(6, 5), Feature::RUINS.tile);
  }

  #[test]
  fn only_dungeon_pois_create_descendable_stairs() {
    let mut area = Area::wilderness(20, 20);
    area.add_poi(PointOfInterest::new((4, 4), PoiKind::Ruins, 1));
    assert!(!area.is_stairs_down(4, 4));
    area.add_poi(PointOfInterest::new((8, 8), PoiKind::Dungeon, 2));
    assert!(area.is_stairs_down(8, 8));
  }

  #[test]
  fn walls_block_and_stairs_are_walkable() {
    let mut area = Area::dungeon(10, 10);
    area.set_wall(2, 2);
    area.set_stairs_down(3, 3);

    assert!(!area.is_walkable(2, 2));
    assert!(area.is_opaque(2, 2));
    assert!(area.is_walkable(3, 3));
    assert!(area.is_stairs_down(3, 3));
    assert_eq!(area.get_tile(3, 3).symbol, '>');
  }
}
