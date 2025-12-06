use std::collections::HashMap;
use anyhow::{anyhow, Result};
use dagr_lib::components::world::{
  dungeon::Dungeon,
  dungeon_connection::{DungeonConnection, ConnectionType},
  dungeon_passage::DungeonPassage,
  dungeon_room::DungeonRoom,
  location::Location,
  spatial::{Spatial, SpatialData},
};
use dagr_lib::ems::component::Component;
use dagr_lib::ems::entity_manager::EntityManager;
use hecs::Entity;
use crate::areas::Area;

pub struct DungeonGenerator{
  pub seed: u64,
}

impl DungeonGenerator{
  pub fn new(seed: u64) -> Self{
    Self{seed}
  }

  pub fn generate(
    &self,
    dungeon_entity: Entity,
    entity_manager: &EntityManager,
    target_level: i32,
  ) -> Result<Area>{
    let dungeon_location = entity_manager.get_component::<Location, _>(dungeon_entity)?;
    let dungeon_location_id = dungeon_location.get().get_id();
    
    let dungeon = entity_manager.get_component::<Dungeon, _>(dungeon_entity)?;
    let max_level = dungeon.get().get_depth_levels();

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

        if room_data.get_is_entrance(){
          entrance_spatial = Some(spatial_data.clone());
        }

        min_x = min_x.min(spatial_data.get_x());
        max_x = max_x.max(spatial_data.get_x() + spatial_data.get_width());
        min_y = min_y.min(spatial_data.get_y());
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

    let mut area = Area::dungeon(width, height);
    area.set_current_level(target_level);
    area.set_max_level(max_level);

    for spatial_data in &rooms{
      let rx = spatial_data.get_x() - min_x;
      let ry = spatial_data.get_y() - min_y;
      let rw = spatial_data.get_width();
      let rh = spatial_data.get_length();

      for x in rx..(rx + rw){
        area.set_wall(x, ry);
        area.set_wall(x, ry+rh-1);
      }

      for y in ry..(ry + rh){
        area.set_wall(rx, y);
        area.set_wall(rx, ry+rh-1);
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
          let from_cx = (from_room.get_x() + from_room.get_width()/2) - min_x;
          let from_cy = (from_room.get_y() + from_room.get_length()/2) - min_y;
          let to_cx = (to_room.get_x() + to_room.get_width()/2) - min_x;
          let to_cy = (to_room.get_y() + to_room.get_length()/2) - min_y;

          let hx_start = from_cx.min(to_cx);
          let hx_end = from_cx.max(to_cx);
          for x in hx_start..=hx_end{
            if from_cy > 0 && !area.is_walkable(x, from_cy-1){
              area.set_wall(x, from_cy-1);
            }

            area.remove_feature(x, from_cy);
            if from_cy + 1 < height && !area.is_walkable(x, from_cy+1){
              area.set_wall(x, from_cy+1);
            }
          }

          let vy_start = from_cy.min(to_cy);
          let vy_end = from_cy.max(to_cy);
          for y in vy_start..=vy_end{
            if from_cy > 0 && !area.is_walkable(to_cx-1 , y){
              area.set_wall(to_cx-1, y);
            }
            area.remove_feature(to_cx, y);
            if to_cx + 1 < width && !area.is_walkable(to_cx+1, y){
              area.set_wall(to_cx+1, y);
            }
          }
        }
      }
    }

    let connection_entities = entity_manager.find_all_children::<DungeonConnection>(dungeon_location_id);
    for entity in connection_entities{
      let connection = entity_manager.get_component::<DungeonConnection, _>(entity)?;
      let cx_data = connection.get();
      if cx_data.get_connection_type() == ConnectionType::StairsDown{
        let from_id = cx_data.get_from_location_id();
        let to_id = cx_data.get_to_location_id();

        if let Some(from_room) = room_spatials.get(&from_id){
          let stair_x = (from_room.get_x() + from_room.get_width()/2) - min_x;
          let stair_y = (from_room.get_y() + from_room.get_length()/2) - min_y;
          area.set_stairs_down(stair_x, stair_y);
        }

        if let Some(to_room) = room_spatials.get(&to_id){
          let stair_x = (to_room.get_x() + to_room.get_width()/2) - min_x;
          let stair_y = (to_room.get_y() + to_room.get_length()/2) - min_y;
          area.set_stairs_up(stair_x, stair_y);
        }
      }
    }

    if target_level == 1{
      if let Some(entrance) = entrance_spatial{
        let entrance_x = (entrance.get_x() + entrance.get_width()/2) - min_x;
        let entrance_y = (entrance.get_y() + entrance.get_length()/2) - min_y;
        area.set_entrance(entrance_x, entrance_y);
        if area.stairs_up.is_none(){
          area.set_stairs_up(entrance_x, entrance_y);
        }
      }
    }

    Ok(area)
  }
}
