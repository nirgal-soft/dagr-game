use anyhow::Result;
use dagr_lib::{
  components::world::hex::HexData,
  kits::hexkit::terrain::{Terrain, Vegetation, Water},
};
use rand::{Rng, SeedableRng, rngs::StdRng};
use crate::areas::{Area, Feature, Ground, Pos};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WildernessProfile {
  pub terrain: Terrain,
  pub vegetation: Vegetation,
  pub water: Water,
}

impl WildernessProfile {
  pub fn from_hex(hex: &HexData) -> Result<Self> {
    Ok(Self {
      terrain: hex.get_terrain()?,
      vegetation: hex.get_vegetation()?,
      water: hex.get_water()?,
    })
  }

  fn ground(self) -> Ground {
    match self.terrain {
      Terrain::Mountains => Ground::MOUNTAIN,
      Terrain::Hills => Ground::HILLS,
      Terrain::Plains => Ground::PLAINS,
      Terrain::Swamp => Ground::SWAMP,
    }
  }

  fn border_feature(self) -> Feature {
    match (self.terrain, self.vegetation) {
      (Terrain::Swamp, _) => Feature::WATER,
      (Terrain::Mountains, Vegetation::Barren) => Feature::ROCK,
      (_, Vegetation::DenseForest | Vegetation::LightForest) => Feature::TREE,
      _ => Feature::ROCK,
    }
  }
}

impl Default for WildernessProfile {
  fn default() -> Self {
    Self {
      terrain: Terrain::Hills,
      vegetation: Vegetation::LightForest,
      water: Water::Dry,
    }
  }
}

pub struct WildernessGenerator {
  pub seed: u64,
  pub profile: WildernessProfile,
}

impl WildernessGenerator {
  pub fn new(seed: u64) -> Self {
    Self { seed, profile: WildernessProfile::default() }
  }

  pub fn for_hex(seed: u64, hex: &HexData) -> Result<Self> {
    Ok(Self { seed, profile: WildernessProfile::from_hex(hex)? })
  }

  pub fn generate(&self, width: i32, height: i32) -> Result<Area> {
    let mut rng = StdRng::seed_from_u64(self.seed);
    let mut area = Area::wilderness(width, height);
    area.set_ground(self.profile.ground());

    self.paint_vegetation(&mut area, &mut rng);
    self.paint_terrain(&mut area, &mut rng);
    self.paint_water(&mut area, &mut rng);
    self.enclose_with_stable_gates(&mut area);
    self.carve_routes(&mut area, &mut rng);
    self.place_landmark(&mut area, &mut rng);

    let center = (width / 2, height / 2);
    clear_radius(&mut area, center, 1);
    area.set_entrance(center.0, center.1);
    Ok(area)
  }

  fn paint_vegetation(&self, area: &mut Area, rng: &mut StdRng) {
    let scale = ((area.width * area.height) / 1200).max(1);
    let (blobs, steps, brush) = match self.profile.vegetation {
      Vegetation::DenseForest => (scale * 4, 28, 2),
      Vegetation::LightForest => (scale * 3, 20, 2),
      Vegetation::Grassland => (scale, 10, 1),
      Vegetation::Barren => (0, 0, 0),
    };
    paint_blobs(area, rng, Feature::TREE, blobs, steps, brush);
  }

  fn paint_terrain(&self, area: &mut Area, rng: &mut StdRng) {
    let scale = ((area.width * area.height) / 1400).max(1);
    let (blobs, steps, brush) = match self.profile.terrain {
      Terrain::Mountains => (scale * 4, 24, 2),
      Terrain::Hills => (scale * 2, 16, 2),
      Terrain::Plains => (scale, 7, 1),
      Terrain::Swamp => (scale, 8, 1),
    };
    paint_blobs(area, rng, Feature::ROCK, blobs, steps, brush);
  }

  fn paint_water(&self, area: &mut Area, rng: &mut StdRng) {
    match self.profile.water {
      Water::Lake => {
        let scale = ((area.width * area.height) / 1800).max(1);
        paint_blobs(area, rng, Feature::WATER, scale * 2, 42, 3);
      }
      Water::River => paint_river(area, rng),
      Water::Dry if self.profile.terrain == Terrain::Swamp => {
        let scale = ((area.width * area.height) / 1600).max(1);
        paint_blobs(area, rng, Feature::WATER, scale * 2, 20, 2);
      }
      Water::Dry => {}
    }
  }

  fn enclose_with_stable_gates(&self, area: &mut Area) {
    let border = self.profile.border_feature();
    for x in 0..area.width {
      area.set_feature(x, 0, border);
      area.set_feature(x, area.height - 1, border);
    }
    for y in 0..area.height {
      area.set_feature(0, y, border);
      area.set_feature(area.width - 1, y, border);
    }
    for gate in edge_gates(area.width, area.height) {
      area.remove_feature(gate.0, gate.1);
    }
  }

  fn carve_routes(&self, area: &mut Area, rng: &mut StdRng) {
    let center = (area.width / 2, area.height / 2);
    for gate in edge_gates(area.width, area.height) {
      carve_winding_path(area, rng, center, gate);
    }
  }

  fn place_landmark(&self, area: &mut Area, rng: &mut StdRng) {
    if area.width < 7 || area.height < 7 {
      return;
    }
    let center = (area.width / 2, area.height / 2);
    let gates = edge_gates(area.width, area.height);
    let gate = gates[rng.random_range(0..gates.len())];
    let landmark = ((center.0 + gate.0) / 2, (center.1 + gate.1) / 2);
    area.set_feature(landmark.0, landmark.1, Feature::LANDMARK);
  }
}

fn paint_blobs(
  area: &mut Area,
  rng: &mut StdRng,
  feature: Feature,
  count: i32,
  steps: i32,
  brush: i32,
) {
  if area.width <= 4 || area.height <= 4 {
    return;
  }
  for _ in 0..count {
    let mut cursor = (
      rng.random_range(2..area.width - 2),
      rng.random_range(2..area.height - 2),
    );
    for _ in 0..steps {
      for dy in -brush..=brush {
        for dx in -brush..=brush {
          if dx * dx + dy * dy <= brush * brush && rng.random_bool(0.72) {
            let x = cursor.0 + dx;
            let y = cursor.1 + dy;
            if x > 0 && x < area.width - 1 && y > 0 && y < area.height - 1 {
              area.set_feature(x, y, feature);
            }
          }
        }
      }
      cursor.0 = (cursor.0 + rng.random_range(-1..=1)).clamp(2, area.width - 3);
      cursor.1 = (cursor.1 + rng.random_range(-1..=1)).clamp(2, area.height - 3);
    }
  }
}

fn paint_river(area: &mut Area, rng: &mut StdRng) {
  if area.width < 5 || area.height < 5 {
    return;
  }
  if rng.random_bool(0.5) {
    let mut y = rng.random_range(2..area.height - 2);
    for x in 1..area.width - 1 {
      area.set_feature(x, y, Feature::WATER);
      if rng.random_bool(0.45) {
        y = (y + rng.random_range(-1..=1)).clamp(1, area.height - 2);
      }
    }
  } else {
    let mut x = rng.random_range(2..area.width - 2);
    for y in 1..area.height - 1 {
      area.set_feature(x, y, Feature::WATER);
      if rng.random_bool(0.45) {
        x = (x + rng.random_range(-1..=1)).clamp(1, area.width - 2);
      }
    }
  }
}

fn edge_gates(width: i32, height: i32) -> [Pos; 4] {
  [
    (width / 2, 0),
    (width - 1, height / 2),
    (width / 2, height - 1),
    (0, height / 2),
  ]
}

fn carve_winding_path(area: &mut Area, rng: &mut StdRng, from: Pos, to: Pos) {
  let mut cursor = from;
  area.remove_feature(cursor.0, cursor.1);
  while cursor != to {
    let move_x = cursor.0 != to.0;
    let move_y = cursor.1 != to.1;
    if move_x && move_y {
      if rng.random_bool(0.5) {
        cursor.0 += (to.0 - cursor.0).signum();
      } else {
        cursor.1 += (to.1 - cursor.1).signum();
      }
    } else if move_x {
      cursor.0 += (to.0 - cursor.0).signum();
    } else {
      cursor.1 += (to.1 - cursor.1).signum();
    }
    area.remove_feature(cursor.0, cursor.1);
  }
}

fn clear_radius(area: &mut Area, center: Pos, radius: i32) {
  for dy in -radius..=radius {
    for dx in -radius..=radius {
      area.remove_feature(center.0 + dx, center.1 + dy);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::collections::{HashMap, VecDeque};

  fn count(area: &Area, feature: Feature) -> usize {
    (0..area.height)
      .flat_map(|y| (0..area.width).map(move |x| (x, y)))
      .filter(|(x, y)| area.get_feature(*x, *y) == Some(&feature))
      .count()
  }

  fn route_exists(area: &Area, start: Pos, goal: Pos) -> bool {
    let mut queue = VecDeque::from([start]);
    let mut seen = HashMap::from([(start, ())]);
    while let Some(pos) = queue.pop_front() {
      if pos == goal {
        return true;
      }
      for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
        let next = (pos.0 + dx, pos.1 + dy);
        if area.is_walkable(next.0, next.1) && !seen.contains_key(&next) {
          seen.insert(next, ());
          queue.push_back(next);
        }
      }
    }
    false
  }

  #[test]
  fn generation_is_deterministic_and_keeps_routes_to_every_edge() {
    let generator = WildernessGenerator::new(1234);
    let first = generator.generate(64, 40).unwrap();
    let second = generator.generate(64, 40).unwrap();

    for y in 0..40 {
      for x in 0..64 {
        assert_eq!(first.get_tile(x, y), second.get_tile(x, y));
      }
    }
    let center = first.entrance.unwrap();
    for gate in edge_gates(first.width, first.height) {
      assert!(route_exists(&first, center, gate), "gate {gate:?} is unreachable");
    }
    assert_eq!(count(&first, Feature::LANDMARK), 1);
  }

  #[test]
  fn parent_profile_materially_changes_generation() {
    let dense = WildernessGenerator {
      seed: 77,
      profile: WildernessProfile {
        terrain: Terrain::Hills,
        vegetation: Vegetation::DenseForest,
        water: Water::Dry,
      },
    }
    .generate(64, 40)
    .unwrap();
    let plains = WildernessGenerator {
      seed: 77,
      profile: WildernessProfile {
        terrain: Terrain::Plains,
        vegetation: Vegetation::Grassland,
        water: Water::Dry,
      },
    }
    .generate(64, 40)
    .unwrap();
    assert!(count(&dense, Feature::TREE) > count(&plains, Feature::TREE));
    assert_ne!(dense.get_tile(10, 10), plains.get_tile(10, 10));
  }

  #[test]
  fn river_and_lake_profiles_create_coherent_water() {
    for water in [Water::River, Water::Lake] {
      let area = WildernessGenerator {
        seed: 99,
        profile: WildernessProfile {
          terrain: Terrain::Plains,
          vegetation: Vegetation::Grassland,
          water,
        },
      }
      .generate(64, 40)
      .unwrap();
      assert!(count(&area, Feature::WATER) > 20);
    }
  }
}
