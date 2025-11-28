pub mod dungeon_area;
pub use dungeon_area::{DungeonArea, DungeonTileType};

use std::collections::HashMap;
use anyhow::{anyhow, Result};
use crossterm::style::Color;
use dagr_lib::components::world::{
  dungeon_connection::DungeonConnection,
  dungeon_passage::DungeonPassage,
  dungeon_room::DungeonRoom,
  location::Location,
  spatial::{Spatial, SpatialData},
};
use dagr_lib::ems::{
  component::Component,
  entity_manager::EntityManager
};
use hecs::Entity;
use rand::{
  rngs::StdRng,
  SeedableRng,
  // Rng
};
use tracing::info;
use crate::tile::Tile;

//temp
use dagr_lib::kits::util_kit::bsp::BSPNode;
use dagr_lib::kits::util_kit::rect::Rect as DRect;

pub struct DungeonGenerator{
  pub seed: u64,
}

impl DungeonGenerator{
  pub fn new(seed: u64) -> Self{
    Self{seed}
  }

  pub fn generate(&self, dungeon_entity: Entity, entity_manager: &EntityManager) -> Result<DungeonArea>{
    info!("building dungeon area");
    let dungeon_location = entity_manager.get_component::<Location, _>(dungeon_entity)?;
    let dungeon_location_id = dungeon_location.get().get_id();
    let target_level = 1;

    info!("dungeon location_id: {}, loading level {}", dungeon_location_id, target_level);
    
    let room_entities = entity_manager.find_all_children::<DungeonRoom>(dungeon_location_id);

    let mut min_x = i32::MAX;
    let mut max_x = i32::MIN;
    let mut min_y = i32::MAX;
    let mut max_y = i32::MIN;

    let mut room_spatials: HashMap<i32, SpatialData> = HashMap::new();
    let mut rooms: Vec<SpatialData> = Vec::new();
    let mut entrance_spatial: Option<SpatialData> = None;

    for entity in room_entities{
      let room = entity_manager.get_component::<DungeonRoom, _>(entity)?;
      let location = entity_manager.get_component::<Location, _>(entity)?;
      let spatial = entity_manager.get_component::<Spatial, _>(entity)?;

      if room.get().get_dungeon_level() == target_level{
        let spatial_data = spatial.get();
        let room_data = room.get();
        let location_id = location.get().get_id();
        info!("found room {} at ({}, {}) size {}x{}",
          location_id,
          spatial_data.get_x(),
          spatial_data.get_y(),
          spatial_data.get_width(),
          spatial_data.get_length()
        );

        if room_data.get_is_entrance(){
          entrance_spatial = Some(spatial_data.clone());
        }

        min_x = min_x.min(spatial_data.get_x());
        min_y = min_y.min(spatial_data.get_y());
        max_x = max_x.max(spatial_data.get_x() + spatial_data.get_width());
        max_y = max_y.max(spatial_data.get_y() + spatial_data.get_length());

        room_spatials.insert(location_id, spatial_data.clone());
        rooms.push(spatial_data);
      }
    }

    if rooms.is_empty(){
      return Err(anyhow!("no rooms found for dungeon level {}", target_level));
    }

    let width = max_x - min_x;
    let height = max_y - min_y;

    info!("dungeon level {} bounds: {}x{} (from {},{} to {},{}",
      target_level, width, height, min_x, min_y, max_x, max_y);

    let mut area = DungeonArea::new(width, height);

    for spatial_data in &rooms{
      let rx = spatial_data.get_x() - min_x;
      let ry = spatial_data.get_y() - min_y;
      let rw = spatial_data.get_width();
      let rh = spatial_data.get_length();

      for x in rx..(rx + rw){
        area.set_tile(x, ry, DungeonTileType::Wall);
        area.set_tile(x, ry + rh - 1, DungeonTileType::Wall);
      }

      for y in ry..(ry + rh){
        area.set_tile(rx, y, DungeonTileType::Wall);
        area.set_tile(rx + rw - 1, y, DungeonTileType::Wall);
      }

      for y in (ry + 1)..(ry + rh - 1){
        for x in (rx + 1)..(rx + rw -1){
          area.set_tile(x, y, DungeonTileType::Floor);
        }
      }
    }

    let passage_entities = entity_manager.find_all_children::<DungeonPassage>(dungeon_location_id);

    for entity in passage_entities{
      let passage = entity_manager.get_component::<DungeonPassage, _>(entity)?;
      let passage_data = passage.get();

      if passage_data.get_dungeon_level() == target_level{
        let from_id = passage_data.get_connects_from();
        let to_id = passage_data.get_connects_to();

        if let (Some(from_room), Some(to_room)) = (room_spatials.get(&from_id), room_spatials.get(&to_id)){
          let from_cx = (from_room.get_x() + from_room.get_width() / 2) - min_x;
          let from_cy = (from_room.get_y() + from_room.get_length() / 2) - min_y;
          let to_cx = (to_room.get_x() + to_room.get_width() / 2) - min_x;
          let to_cy = (to_room.get_y() + to_room.get_length() / 2) - min_y;

          info!("drawing L-corridor from room {} ({},{}) to room {} ({},{})",
            from_id, from_cx, from_cy, to_id, to_cx, to_cy);

          let hx_start = from_cx.min(to_cx);
          let hx_end = from_cx.max(to_cx);
          for x in hx_start..=hx_end{
            if from_cy > 0 && !area.is_walkable(x, from_cy - 1){
              area.set_tile(x, from_cy - 1, DungeonTileType::Wall);
            }
            area.set_tile(x, from_cy, DungeonTileType::Floor);
            if from_cy + 1 < height && !area.is_walkable(x, from_cy +1){
              area.set_tile(x, from_cy + 1, DungeonTileType::Wall);
            }
          }

          let vy_start = from_cy.min(to_cy);
          let vy_end = from_cy.max(to_cy);
          for y in vy_start..=vy_end{
            if to_cx > 0 && !area.is_walkable(to_cx - 1, y){
              area.set_tile(to_cx -1, y, DungeonTileType::Wall);
            }
            area.set_tile(to_cx, y, DungeonTileType::Floor);
            if to_cx + 1 < width && !area.is_walkable(to_cx + 1, y){
              area.set_tile(to_cx + 1, y, DungeonTileType::Wall);
            }
          }
        }else{
          info!("warning: passage connects rooms {} -> {} but one or both not found", from_id, to_id);
        }
      }
    }

    if let Some(entrance) = entrance_spatial{
      let entrance_x = (entrance.get_x() + entrance.get_width() / 2) - min_x;
      let entrance_y = (entrance.get_y() + entrance.get_length() / 2) - min_y;
      area.set_entrance(entrance_x, entrance_y);
      area.set_tile(entrance_x, entrance_y, DungeonTileType::StairsUp);
      info!("dungeon entrance at {},{}", entrance_x, entrance_y);
    }

    Ok(area)
  }

  pub fn generate_raw(
    &self,
    width: i32,
    height: i32,
    min_room_size: i32,
    max_depth: i32
  ) -> Result<DungeonArea>{
    let mut rng = StdRng::seed_from_u64(self.seed);

    let mut root = BSPNode::new(DRect{
      x: 0,
      y: 0,
      w: width,
      h: height,
    });
    root.split(&mut rng, min_room_size, max_depth, 0);
    root.create_rooms(&mut rng)?;

    let mut passages = Vec::new();
    root.create_passages(&mut passages);

    let mut area = DungeonArea::new(width, height);

    let mut rooms = Vec::new();
    collect_rooms(&root, &mut rooms);

    set_room_tiles(&mut area, &rooms);
    set_passage_tiles(&mut area, &passages);

    Ok(area)
  }
}

fn collect_rooms(node: &BSPNode, rooms: &mut Vec<DRect>){
  if let Some(room) = &node.room{
    rooms.push(room.clone());
  }
  if let Some(left) = &node.left{
    collect_rooms(left, rooms);
  }
  if let Some(right) = &node.right{
    collect_rooms(right, rooms);
  }
}

fn set_room_tiles(area: &mut DungeonArea, rooms: &Vec<DRect>){
  for room in rooms{
    for y in room.y..(room.y+room.h){
      area.set_tile(room.x, y, DungeonTileType::Wall);
      area.set_tile(room.x + room.w - 1, y, DungeonTileType::Wall);
    }
    for x in room.x..(room.x+room.w){
      area.set_tile(x, room.y, DungeonTileType::Wall);
      area.set_tile(x, room.y + room.h - 1, DungeonTileType::Wall);
    }

    for y in (room.y + 1)..(room.y + room.h - 1){
      for x in (room.x+1)..(room.x+room.w-1){
        area.set_tile(x, y, DungeonTileType::Floor);
      }
    }
  }
}

fn set_passage_tiles(area: &mut DungeonArea, passages: &Vec<DRect>){
  for passage in passages{
    if passage.w == 1{
      for y in passage.y..(passage.y+passage.h){
        if passage.x > 0 && area.get_tile(passage.x - 1, y).is_none(){
          area.set_tile(passage.x-1, y, DungeonTileType::Wall);
        }
        area.set_tile(passage.x, y, DungeonTileType::Floor);
        if passage.x + 1 < area.width && area.get_tile(passage.x + 1, y).is_none(){
          area.set_tile(passage.x+1, y, DungeonTileType::Wall);
        }
      }
    }else if passage.h == 1{
      for x in passage.x..(passage.x + passage.w){
        if passage.y > 0 && area.get_tile(x, passage.y - 1).is_none(){
          area.set_tile(x, passage.y - 1, DungeonTileType::Wall);
        }
        area.set_tile(x, passage.y, DungeonTileType::Floor);
        if passage.y + 1 < area.height && area.get_tile(x, passage.y + 1).is_none(){
          area.set_tile(x, passage.y + 1, DungeonTileType::Wall);
        }
      }
    }else{
      for y in passage.y..(passage.y + passage.h){
        if area.get_tile(passage.x, y).is_none(){
          area.set_tile(passage.x, y , DungeonTileType::Wall);
        }
        if area.get_tile(passage.x + passage.w - 1, y).is_none(){
          area.set_tile(passage.x + passage.w - 1, y, DungeonTileType::Wall);
        }
      }
      for x in passage.x..(passage.x + passage.w){
        if area.get_tile(x, passage.y).is_none(){
          area.set_tile(x, passage.y, DungeonTileType::Wall);
        }
        if area.get_tile(x, passage.y + passage.h - 1).is_none(){
          area.set_tile(x, passage.y + passage.h - 1, DungeonTileType::Wall);
        }
      }
      for y in (passage.y+1)..(passage.y+passage.h-1){
        for x in (passage.x+1)..(passage.x+passage.w-1){
          area.set_tile(x, y, DungeonTileType::Floor);
        }
      }
    }
  }
}
