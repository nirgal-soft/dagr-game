pub mod dungeon_area;
pub use dungeon_area::DungeonArea;

use std::collections::HashMap;
use anyhow::{anyhow, Result};
use crossterm::style::Color;
use dagr_lib::components::world::{
  dungeon_connection::DungeonConnection,
  dungeon_passage::DungeonPassage,
  dungeon_room::DungeonRoom,
  location::Location,
  spatial::Spatial
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
    let mut rooms = Vec::new();
    let mut entrance_x = 0;
    let mut entrance_y = 0;
    let mut entrance_found = false;

    for entity in room_entities{
      let room = entity_manager.get_component::<DungeonRoom, _>(entity)?;
      let spatial = entity_manager.get_component::<Spatial, _>(entity)?;

      if room.get().get_dungeon_level() == target_level{
        let spatial_data = spatial.get();
        let room_data = room.get();
        info!("found room at ({}, {})", spatial_data.get_x(), spatial_data.get_y());

        if room_data.get_is_entrance() && !entrance_found{
          entrance_x = spatial_data.get_x() + spatial_data.get_width() / 2;
          entrance_y = spatial_data.get_y() + spatial_data.get_length() / 2;
          entrance_found = true;
        }

        min_x = min_x.min(spatial_data.get_x());
        min_y = min_y.min(spatial_data.get_y());
        max_x = max_x.max(spatial_data.get_x() + spatial_data.get_width());
        max_y = max_y.max(spatial_data.get_y() + spatial_data.get_length());

        rooms.push(spatial_data);
      }
    }

    if rooms.is_empty(){
      return Err(anyhow!("no rooms found for dungeon level {}", target_level));
    }

    if !entrance_found{
      entrance_x = rooms[0].get_x() + rooms[0].get_width() / 2;
      entrance_y = rooms[0].get_y() + rooms[0].get_length() / 2;
    }

    let width = max_x - min_x;
    let height = max_y - min_y;

    info!("dungeon level {} bounds: {}x{} (from {},{} to {},{}",
      target_level, width, height, min_x, min_y, max_x, max_y);

    let mut area = DungeonArea::new(width, height);

    // for y in 0..height{
    //   for x in 0..width{
    //     area.set_tile(x, y, Tile{
    //       symbol: '#',
    //       color: Color::DarkGrey,
    //     })
    //   }
    // }

    for spatial_data in rooms{
      let rx = spatial_data.get_x() - min_x;
      let ry = spatial_data.get_y() - min_y;
      let rw = spatial_data.get_width();
      let rh = spatial_data.get_length();

      for y in ry..(ry + rh){
        for x in rx..(rx + rw){
          area.set_tile(x, ry, Tile{
            symbol: '#',
            color: Color::White,
          });
          area.set_tile(x, ry + rh - 1, Tile{
            symbol: '#',
            color: Color::White,
          });
        }

        for y in ry..(ry+rh){
          area.set_tile(rx, y, Tile{
            symbol: '#',
            color: Color::White,
          });
          area.set_tile(rx + rw - 1, y, Tile{
            symbol: '#',
            color: Color::White,
          });
        }

        for y in (ry + 1)..(ry + rh - 1){
          for x in (rx + 1)..(rx + rw - 1){
            area.set_tile(x, y, Tile{
              symbol: '.',
              color: Color::White,
            });
          }
        }
      }
    }

    let passage_entities = entity_manager.find_all_children::<DungeonPassage>(dungeon_location_id);

    for entity in passage_entities{
      let passage = entity_manager.get_component::<DungeonPassage, _>(entity)?;
      let spatial = entity_manager.get_component::<Spatial, _>(entity)?;

      if passage.get().get_dungeon_level() == target_level{
        let spatial_data = spatial.get();
        info!("found passage at ({}, {})", spatial_data.get_x(), spatial_data.get_y());

        let px = spatial_data.get_x() - min_x;
        let py = spatial_data.get_y() - min_y;
        let pw = spatial_data.get_width();
        let ph = spatial_data.get_length();

        for y in py..(py + ph){
          for x in px..(px + pw){
            area.set_tile(x, y, Tile{
              symbol: '.',
              color: Color::White,
            });
          }
        }
      }
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

    let leaf_rooms = root.get_leaf_rooms();
    let mut passages = Vec::new();
    root.create_passages(&mut passages);

    let mut area = DungeonArea::new(width, height);

    for leaf_rect in &leaf_rooms{
      if let Some(room) = root.get_room(){
        for y in room.y..(room.y + room.h){
          for x in room.x..(room.x + room.w){
            if x == room.x || x == room.x + room.w - 1 ||
              y == room.y || y == room.y + room.h - 1{
              area.set_tile(x, y, Tile{
                symbol: '#',
                color: Color::White,
              });
            }else{
              area.set_tile(x, y, Tile{
                symbol: '.',
                color: Color::White,
              });
            }
          }
        }
      }
    }

    for passage in &passages{
      for y in passage.y..(passage.y + passage.h){
        for x in passage.x..(passage.x + passage.w){
          area.set_tile(x, y, Tile{
            symbol: '.',
            color: Color::White,
          });
        }
      }
    }

    Ok(area)
  }
}
