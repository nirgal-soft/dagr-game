use anyhow::Result;
use rand::{Rng, SeedableRng, rngs::StdRng};
use std::collections::HashMap;

use crate::areas::{Area, Feature};

pub struct WildernessGenerator {
  pub seed: u64,
}

impl WildernessGenerator {
  pub fn new(seed: u64) -> Self {
    Self { seed }
  }

  pub fn generate(&self, width: i32, height: i32) -> Result<Area> {
    let mut rng = StdRng::seed_from_u64(self.seed);
    let mut area = Area::wilderness(width, height);

    for y in 0..height {
      for x in 0..width {
        if let Some(feature) = self.generate_feature(&mut rng, x, y, width, height) {
          area.set_feature(x, y, feature);
        }
      }
    }

    self.smooth_features(&mut area, width, height);

    // Always leave a usable arrival zone in the center of the wilderness.
    let center = (width / 2, height / 2);
    for dy in -1..=1 {
      for dx in -1..=1 {
        area.remove_feature(center.0 + dx, center.1 + dy);
      }
    }
    area.set_entrance(center.0, center.1);

    Ok(area)
  }

  fn generate_feature(
    &self,
    rng: &mut StdRng,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
  ) -> Option<Feature> {
    let roll: f32 = rng.random();
    let edge_dist = x.min(y).min(width - x - 1).min(height - y - 1);
    let edge_factor = if edge_dist < 3 { 0.2 } else { 0.0 };

    match roll {
      r if r < 0.15 + edge_factor => Some(Feature::TREE),
      r if r < 0.25 => Some(Feature::WATER),
      r if r < 0.35 + edge_factor => Some(Feature::ROCK),
      _ => None,
    }
  }

  fn smooth_features(&self, area: &mut Area, width: i32, height: i32) {
    let mut changes: Vec<(i32, i32, Option<Feature>)> = Vec::new();

    for y in 0..height {
      for x in 0..width {
        let neighbors = self.count_neighbor_features(area, x, y, width, height);

        if let Some((feature, count)) = neighbors.iter().max_by_key(|(_, c)| *c) {
          if *count >= 5 {
            changes.push((x, y, Some(*feature)));
          }
        }
      }
    }

    for (x, y, feature) in changes {
      match feature {
        Some(f) => area.set_feature(x, y, f),
        None => {
          area.remove_feature(x, y);
        }
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
  ) -> Vec<(Feature, usize)> {
    let mut counts: HashMap<FeatureKey, usize> = HashMap::new();

    for dy in -1..=1 {
      for dx in -1..=1 {
        if dx == 0 && dy == 0 {
          continue;
        }

        let nx = x + dx;
        let ny = y + dy;

        if nx >= 0 && nx < width && ny >= 0 && ny < height {
          if let Some(feature) = area.get_feature(nx, ny) {
            let key = FeatureKey::from(*feature);
            *counts.entry(key).or_insert(0) += 1;
          }
        }
      }
    }

    counts
      .into_iter()
      .map(|(k, c)| (k.into_feature(), c))
      .collect()
  }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct FeatureKey {
  symbol: char,
}

impl FeatureKey {
  fn from(feature: Feature) -> Self {
    Self {
      symbol: feature.tile.symbol,
    }
  }

  fn into_feature(self) -> Feature {
    match self.symbol {
      'T' => Feature::TREE,
      '~' => Feature::WATER,
      'o' => Feature::ROCK,
      _ => Feature::ROCK,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn generation_is_deterministic_and_keeps_the_center_open() {
    let first = WildernessGenerator::new(1234).generate(20, 20).unwrap();
    let second = WildernessGenerator::new(1234).generate(20, 20).unwrap();

    for y in 0..20 {
      for x in 0..20 {
        assert_eq!(first.get_tile(x, y), second.get_tile(x, y));
      }
    }

    assert_eq!(first.entrance, Some((10, 10)));
    for dy in -1..=1 {
      for dx in -1..=1 {
        assert!(first.is_walkable(10 + dx, 10 + dy));
      }
    }
  }
}
