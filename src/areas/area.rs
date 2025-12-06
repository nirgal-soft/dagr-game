use std::collections::{HashMap, HashSet};
use crate::renderer::{Tile, RenderConfig};
use crate::visiblity::Visibility;
use super::ground::Ground;
use super::feature::Feature;

pub type Pos = (i32, i32);

#[derive(Debug)]
pub struct Area{
  pub width: i32,
  pub height: i32,
  ground: Ground,
  features: HashMap<Pos, Feature>,
  seen: HashSet<Pos>,
  pub entrance: Option<Pos>,
  pub stairs_up: Option<Pos>,
  pub stairs_down: Option<Pos>,
  pub current_level: Option<i32>,
  pub max_level: Option<i32>,
}

impl Area{
  pub fn new(width: i32, height: i32, ground: Ground) -> Self{
    Self{
      width,
      height,
      ground,
      features: HashMap::new(),
      seen: HashSet::new(),
      entrance: None,
      stairs_up: None,
      stairs_down: None,
      current_level: None,
      max_level: None,
    }
  }

  pub fn dungeon(width: i32, height: i32) -> Self{
    Self::new(width, height, Ground::DUNGEON)
  }

  pub fn wilderness(width: i32, height: i32) -> Self{
    Self::new(width, height, Ground::WILDERNESS)
  }

  //---accessors and mutators---
  pub fn set_current_level(&mut self, level: i32){self.current_level = Some(level);}
  pub fn set_max_level(&mut self, level: i32){self.max_level = Some(level);}
  pub fn set_feature(&mut self, x: i32, y: i32, feature: Feature){self.features.insert((x, y), feature);}
  pub fn get_feature(&self, x: i32, y: i32) -> Option<&Feature>{self.features.get(&(x, y))}
  pub fn remove_feature(&mut self, x: i32, y: i32){self.features.remove(&(x, y));}

  //---convience mutators for common features---
  pub fn set_wall(&mut self, x: i32, y: i32){self.set_feature(x, y, Feature::WALL);}
  pub fn set_tree(&mut self, x: i32, y: i32){self.set_feature(x, y, Feature::TREE);}
  pub fn set_rock(&mut self, x: i32, y: i32){self.set_feature(x, y, Feature::ROCK);}
  pub fn set_water(&mut self, x: i32, y: i32){self.set_feature(x, y, Feature::WATER);}
  pub fn set_entrance(&mut self, x: i32, y: i32){self.entrance = Some((x, y));}
  pub fn set_stairs_up(&mut self, x: i32, y: i32){self.stairs_up = Some((x, y));}
  pub fn set_stairs_down(&mut self, x: i32, y: i32){self.stairs_down = Some((x, y));}

  //---spatial queries---
  pub fn in_bounds(&self, x: i32, y: i32) -> bool{
    x >= 0 && x < self.width && y >= 0 && y < self.height
  }

  pub fn is_walkable(&self, x: i32, y: i32) -> bool{
    if !self.in_bounds(x, y){
      return false;
    }
    self.features.get(&(x, y)).is_none_or(|f| f.walkable)
  }

  pub fn is_opaque(&self, x: i32, y: i32) -> bool{
    if !self.in_bounds(x, y){
      return true;
    }
    self.features.get(&(x, y)).is_none_or(|f| f.opaque)
  }

  pub fn is_stairs_up(&self, x: i32, y: i32) -> bool{
    self.features.get(&(x, y)).is_none_or(|f| f.is_stairs_up())
  }

  pub fn is_stairs_down(&self, x: i32, y: i32) -> bool{
    self.features.get(&(x, y)).is_none_or(|f| f.is_stairs_down())
  }

  //---visibility---
  pub fn mark_seen(&mut self, x: i32, y: i32){
    self.seen.insert((x, y));
  }

  pub fn is_seen(&self, x: i32, y: i32) -> bool{
    self.seen.contains(&(x, y))
  }

  pub fn mark_visible_as_seen<I>(&mut self, tiles: I)
  where
    I: IntoIterator<Item = Pos>,
  {
    self.seen.extend(tiles);
  }

  pub fn seen_count(&self) -> usize{
    self.seen.len()
  }

  pub fn is_seen_and_walkable(&self, x: i32, y: i32) -> bool{
    self.is_walkable(x, y) && self.seen.contains(&(x, y))
  }

  //---rendering---
  pub fn get_tile(&self, x: i32, y: i32) -> Option<Tile>{
    if !self.in_bounds(x, y){
      return None;
    }

    Some(
      self.features
        .get(&(x, y))
        .map(|f| f.tile)
        .unwrap_or(self.ground.tile)
    )
  }

  pub fn get_visible_tile(
    &self,
    x: i32,
    y: i32,
    currently_visible: bool,
    config: &RenderConfig,
  ) -> Option<Tile>{
    if !self.in_bounds(x, y){
      return None;
    }

    let visibility = if currently_visible{
      Visibility::Visible
    }else if self.seen.contains(&(x, y)){
      Visibility::Seen
    }else{
      Visibility::Unseen
    };

    match visibility{
      Visibility::Unseen => Some(Tile{
        symbol: config.unseen_symbol,
        fg: config.unseen_fg,
        bg: config.unseen_bg
      }),
      Visibility::Seen | Visibility::Visible => {
        let base_tile = self.features
          .get(&(x, y))
          .map(|f| f.tile)
          .unwrap_or(self.ground.tile);
        Some(base_tile.with_visibility(visibility, config))
      }
    }
  }

  //---stair finding---
  pub fn find_seen_stairs_up(&self) -> Vec<Pos>{
    self.features
      .iter()
      .filter(|((x, y), f)| f.is_stairs_up() && self.seen.contains(&(*x, *y)))
      .map(|((x, y), _)| (*x, *y))
      .collect()
  }

  pub fn find_seen_stairs_down(&self) -> Vec<Pos>{
    self.features
      .iter()
      .filter(|((x, y), f)| f.is_stairs_down() && self.seen.contains(&(*x, *y)))
      .map(|((x, y), _)| (*x, *y))
      .collect()
  }

  //--exploration---
  pub fn find_exploration_frontiers(&self) -> Vec<Pos>{
    let mut frontiers = Vec::new();

    for &(x, y) in &self.seen{
      if !self.is_walkable(x, y){
        continue;
      }

      let neighbors = [
        (x-1, y-1), (x, y-1), (x+1, y-1),
        (x-1, y), (x+1, y),
        (x-1, y+1), (x, y+1), (x+1, y+1),
      ];

      for (nx, ny) in neighbors{
        if self.in_bounds(nx, ny) && !self.seen.contains(&(nx, ny)){
          frontiers.push((x, y));
          break;
        }
      }
    }

    frontiers
  }

  pub fn is_fully_explore(&self) -> bool{
    self.find_exploration_frontiers().is_empty()
  }
}
