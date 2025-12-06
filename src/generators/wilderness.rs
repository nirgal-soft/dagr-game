use std::collections::HashMap;
use anyhow::Result;
use rand::{rngs::StdRng, Rng, SeedableRng};

use crate::areas::{Area, Feature};

pub struct WildernessGenerator{
  pub seed: u64,
}

impl WildernessGenerator{
  pub fn new(seed: u64) -> Self{
    Self{
      seed,
    }
  }

  pub fn generate(&self, width: i32, height: i32) -> Result<Area>{
    let mut rng = StdRng::seed_from_u64(self.seed);
    let mut area = Area::wilderness(width, height);

    Ok(area)
  }

  fn generate_feature(
    &self,
    rng: &mut StdRng,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
  ) -> Option<Feature>{
    let roll: f32 = rng.random();
    let edge_dist = x.min(y).min(width-x-1).min(height-y-1);
    let edge_factor = if edge_dist < 3 {0.2} else {0.0};

    match roll{
      r if r < 0.15 + edge_factor => Some(Feature::TREE),
      r if r < 0.25 => Some(Feature::WATER),
      r if r < 0.35 + edge_factor => Some(Feature::ROCK),
      _ => None,
    }
  }

  fn smooth_features(&self, area: &mut Area, width: i32, height: i32){
    let mut changes: Vec<(i32, i32, Option<Feature>)> = Vec::new();

    for y in 0..height{
      for x in 0..width{
        let neighbors = self.count_neighbor_features(area, x, y, width, height);

        if let Some((feature, count)) = neighbors.iter().max_by_key(|(_, c)| *c){
          if *count >= 5{
            changes.push((x, y, Some(*feature)));
          }
        }
      }
    }

    for (x, y, feature) in changes{
      match feature{
        Some(f) => area.set_feature(x, y, f),
        None => {area.remove_feature(x, y);}
      }
    }
  }

  fn count_neighbor_features(
    &self,
    area: &Area,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
  ) -> Vec<(Feature, usize)>{
    let mut counts: HashMap<FeatureKey, usize> = HashMap::new();

    for dy in -1..=1{
      for dx in -1..=1{
        if dx == 0 && dy == 0{
          continue;
        }

        let nx = x + dx;
        let ny = y + dy;

        if nx >= 0 && nx < width && ny >= 0 && ny < height{
          if let Some(feature) = area.get_feature(nx, ny){
            let key = FeatureKey::from(*feature);
            *counts.entry(key).or_insert(0) += 1;
          }
        }
      }
    }

    counts
      .into_iter()
      .map(|(k, c)| (k.to_feature(), c))
      .collect()
  }

  pub fn set_dungeon_entrance(&self, area: &mut Area, x: i32, y: i32){
    area.set_stairs_down(x, y);
  }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct FeatureKey{
  symbol: char,
}

impl FeatureKey{
  fn from(feature: Feature) -> Self{
    Self{
      symbol: feature.tile.symbol,
    }
  }

  fn to_feature(&self) -> Feature{
    match self.symbol{
      'T' => Feature::TREE,
      'W' => Feature::WATER,
      'R' => Feature::ROCK,
      _ => Feature::ROCK,
    }
  }
}
