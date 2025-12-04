use std::collections::{HashMap, HashSet};
use crossterm::style::Color;
use crate::renderer::{Tile, RenderConfig};
use crate::pathfinding::Pos;
use crate::visiblity::Visibility;

pub enum DungeonTileType{
  Floor,
  Wall,
  StairsUp,
  StairsDown,
}

#[derive(Debug)]
pub struct DungeonArea{
  pub width: i32,
  pub height: i32,
  pub entrance: Option<(i32, i32)>,
  pub current_level: i32,
  pub max_level: i32,
  pub stairs_up: Option<(i32, i32)>,
  pub stairs_down: Option<(i32, i32)>,
  tiles: HashMap<(i32, i32), Tile>,
  seen: HashSet<(i32, i32)>,
}

impl DungeonArea{
  pub fn new(width: i32, height: i32) -> Self{
    Self{
      width,
      height,
      entrance: None,
      current_level: 1,
      max_level: 1,
      stairs_up: None,
      stairs_down: None,
      tiles: HashMap::new(),
      seen: HashSet::new(),
    }
  }

  pub fn set_entrance(&mut self, x: i32, y: i32){
    self.entrance = Some((x, y));
  }

  pub fn set_current_level(&mut self, level: i32){
    self.current_level = level;
  }

  pub fn set_max_level(&mut self, level: i32){
    self.max_level = level;
  }

  pub fn set_stairs_up(&mut self, x: i32, y: i32){
    self.stairs_up = Some((x, y));
  }

  pub fn set_stairs_down(&mut self, x: i32, y: i32){
    self.stairs_down = Some((x, y));
  }

  pub fn get_tile(&self, x: i32, y: i32)-> Option<&Tile>{
    self.tiles.get(&(x, y))
  }

  pub fn set_tile(&mut self, x: i32, y: i32, tile_type: DungeonTileType){
    let tile = match tile_type{
      DungeonTileType::Floor => Tile::new('.', Color::Grey)
        .with_bg(Color::Rgb{r: 20, g: 20, b: 25}),
      DungeonTileType::Wall => Tile::new('#', Color::White)
        .with_bg(Color::Rgb{r: 40, g: 35, b: 30}),
      DungeonTileType::StairsUp => Tile::new('<', Color::Yellow)
        .with_bg(Color::Rgb{r: 50, g: 40, b: 20}),
      DungeonTileType::StairsDown => Tile::new('>', Color::Yellow)
        .with_bg(Color::Rgb{r: 50, g: 40, b: 20}),
    };
    self.tiles.insert((x, y), tile);
  }

  pub fn is_walkable(&self, x: i32, y: i32) -> bool{
    if let Some(tile) = self.tiles.get(&(x, y)){
      ['.', '<', '>'].contains(&tile.symbol)
    }else{
      false
    }
  }

  pub fn is_opaque(&self, x: i32, y: i32) -> bool{
    if let Some(tile) = self.tiles.get(&(x, y)){
      tile.symbol == '#'
    }else{
      true
    }
  }

  pub fn is_stairs_up(&self, x: i32, y: i32) -> bool{
    if let Some(tile) = self.tiles.get(&(x, y)){
      tile.symbol == '<'
    }else{
      false
    }
  }

  pub fn is_stairs_down(&self, x: i32, y: i32) -> bool{
    if let Some(tile) = self.tiles.get(&(x, y)){
      tile.symbol == '>'
    }else{
      false
    }
  }

  pub fn mark_seen(&mut self, x: i32, y: i32){
    self.seen.insert((x, y));
  }

  pub fn is_seen(&self, x: i32, y: i32) -> bool{
    self.seen.contains(&(x, y))
  }

  pub fn mark_visible_as_seen<I>(&mut self, tiles: I)
  where
    I: IntoIterator<Item = (i32, i32)>
  {
    for (x, y) in tiles{
      self.seen.insert((x, y));
    }
  }

  pub fn get_visible_tile(
    &self,
    x: i32,
    y: i32,
    currently_visible: bool,
    config: &RenderConfig
  ) -> Option<Tile>{
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
        let tile = self.tiles.get(&(x, y))?;
        Some(tile.with_visibility(visibility, config))
      }
    }
  }

  pub fn find_seen_stairs_up(&self) -> Vec<Pos>{
    self.tiles
      .iter()
      .filter(|((x, y), tile)| tile.symbol == '<' && self.seen.contains(&(*x, *y)))
      .map(|((x, y), _)| (*x, *y))
      .collect()
  }

  pub fn find_seen_stairs_down(&self) -> Vec<Pos>{
    self.tiles
      .iter()
      .filter(|((x, y), tile)| tile.symbol == '>' && self.seen.contains(&(*x, *y)))
      .map(|((x, y), _)| (*x, *y))
      .collect()
  }

  pub fn is_seen_and_walkable(&self, x: i32, y: i32) -> bool{
    self.is_walkable(x, y) && self.seen.contains(&(x, y))
  }

  pub fn find_exploration_frontiers(&self) -> Vec<Pos>{
    let mut frontiers = Vec::new();

    for &(x, y) in self.seen.iter(){
      if !self.is_walkable(x, y){
        continue;
      }

      let neighbors = [
        (x-1, y-1), (x, y-1), (x+1, y-1),
        (x-1, y), (x+1, y),
        (x-1, y+1), (x, y+1), (x+1, y+1),
      ];

      for (nx, ny) in neighbors{
        if self.tiles.contains_key(&(nx, ny)) && !self.seen.contains(&(nx, ny)){
          frontiers.push((x, y));
          break;
        }
      }
    }

    frontiers
  }

  pub fn is_fully_explored(&self) -> bool{
    self.find_exploration_frontiers().is_empty()
  }

  pub fn seen_count(&self) -> usize{
    self.seen.len()
  }
}
