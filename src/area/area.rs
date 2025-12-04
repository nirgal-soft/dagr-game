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
