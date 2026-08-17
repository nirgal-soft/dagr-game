use std::collections::HashMap;

use anyhow::{Result, anyhow};
use dagr_lib::world::{ConnectionType, DungeonView, LocationId};

use crate::areas::Area;

pub struct DungeonGenerator;

impl DungeonGenerator {
  pub fn new(_seed: u64) -> Self {
    Self
  }

  pub fn generate(&self, dungeon: &DungeonView, target_level: i32) -> Result<Area> {
    let rooms = dungeon
      .rooms
      .iter()
      .filter(|room| room.level == target_level)
      .collect::<Vec<_>>();
    if rooms.is_empty() {
      return Err(anyhow!("no rooms found for dungeon level {target_level}"));
    }

    let min_x = rooms
      .iter()
      .map(|room| room.spatial.coordinate.x)
      .min()
      .unwrap();
    let min_y = rooms
      .iter()
      .map(|room| room.spatial.coordinate.y)
      .min()
      .unwrap();
    let max_x = rooms
      .iter()
      .map(|room| room.spatial.coordinate.x + room.spatial.dimensions.width())
      .max()
      .unwrap();
    let max_y = rooms
      .iter()
      .map(|room| room.spatial.coordinate.y + room.spatial.dimensions.length())
      .max()
      .unwrap();
    let width = max_x - min_x;
    let height = max_y - min_y;
    let room_spatials = rooms
      .iter()
      .map(|room| (room.location.id, &room.spatial))
      .collect::<HashMap<LocationId, _>>();

    let mut area = Area::dungeon(width, height);
    area.set_current_level(target_level);
    area.set_max_level(dungeon.depth_levels);
    for y in 0..height {
      for x in 0..width {
        area.set_wall(x, y);
      }
    }

    for room in &rooms {
      let spatial = &room.spatial;
      let rx = spatial.coordinate.x - min_x;
      let ry = spatial.coordinate.y - min_y;
      let rw = spatial.dimensions.width();
      let rh = spatial.dimensions.length();
      for x in rx..rx + rw {
        area.set_wall(x, ry);
        area.set_wall(x, ry + rh - 1);
      }
      for y in ry..ry + rh {
        area.set_wall(rx, y);
        area.set_wall(rx + rw - 1, y);
      }
      for y in ry + 1..ry + rh - 1 {
        for x in rx + 1..rx + rw - 1 {
          area.remove_feature(x, y);
        }
      }
    }

    for passage in dungeon
      .passages
      .iter()
      .filter(|passage| passage.level == target_level)
    {
      let (Some(from), Some(to)) = (
        room_spatials.get(&passage.connects_from),
        room_spatials.get(&passage.connects_to),
      ) else {
        continue;
      };
      let from_x = from.coordinate.x + from.dimensions.width() / 2 - min_x;
      let from_y = from.coordinate.y + from.dimensions.length() / 2 - min_y;
      let to_x = to.coordinate.x + to.dimensions.width() / 2 - min_x;
      let to_y = to.coordinate.y + to.dimensions.length() / 2 - min_y;
      for x in from_x.min(to_x)..=from_x.max(to_x) {
        area.remove_feature(x, from_y);
      }
      for y in from_y.min(to_y)..=from_y.max(to_y) {
        area.remove_feature(to_x, y);
      }
    }

    for connection in dungeon
      .connections
      .iter()
      .filter(|connection| connection.connection_type == ConnectionType::StairsDown)
    {
      if let Some(from) = room_spatials.get(&connection.from) {
        area.set_stairs_down(from.coordinate.x - min_x, from.coordinate.y - min_y);
      }
      if let Some(to) = room_spatials.get(&connection.to) {
        area.set_stairs_up(to.coordinate.x - min_x, to.coordinate.y - min_y);
      }
    }

    if target_level == 1 {
      if let Some(entrance) = rooms.iter().find(|room| room.is_entrance) {
        let x = entrance.spatial.coordinate.x - min_x;
        let y = entrance.spatial.coordinate.y - min_y;
        area.set_entrance(x, y);
        if area.stairs_up.is_none() {
          area.set_stairs_up(x, y);
        }
      }
    }
    Ok(area)
  }
}
