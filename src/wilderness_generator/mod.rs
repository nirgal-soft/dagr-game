pub mod wilderness_area;
pub mod wilderness_tile;

pub use wilderness_area::WildernessArea;

use std::collections::HashMap;
use anyhow::Result;
use rand::{
  rngs::StdRng,
  SeedableRng,
  Rng
};
use wilderness_tile::{WildernessTileType, WildernessTile};

pub struct WildernessGenerator{
  pub seed: u64,
}

impl WildernessGenerator{
  pub fn new(seed: u64) -> Self{
    Self{seed}
  }

  pub fn generate(&self, width: i32, height: i32) -> Result<WildernessArea>{
    let mut rng = StdRng::seed_from_u64(self.seed);
    let mut tiles: Vec<Vec<WildernessTile>> = Vec::with_capacity(height as usize);

    for y in 0..height{
      let mut row = Vec::with_capacity(width as usize);
      for x in 0..width{
        let tile_type = self.generate_tile(&mut rng, x, y, width, height);
        row.push(WildernessTile::new(tile_type));
      }
      tiles.push(row);
    }

    tiles = self.smooth_tiles(tiles, width, height);

    Ok(WildernessArea{
      width,
      height,
      tiles,
    })
  }

  fn generate_tile(&self, rng: &mut StdRng, x: i32, y: i32, w: i32, h: i32) -> WildernessTileType{
    let roll: f32 = rng.random();

    let edge_dist = x.min(y).min(w-x-1).min(h-y-1);
    let edge_factor = if edge_dist < 3 {0.2} else {0.0};

    match roll{
      r if r < 0.15 + edge_factor => WildernessTileType::Tree,
      r if r < 0.25 => WildernessTileType::Water,
      r if r < 0.35 + edge_factor => WildernessTileType::Rock,
      r if r < 0.45 => WildernessTileType::Dirt,
      _ => WildernessTileType::Grass,
    }
  }

  fn smooth_tiles(&self, tiles: Vec<Vec<WildernessTile>>, w: i32, h: i32) -> Vec<Vec<WildernessTile>>{
    let mut smoothed = tiles.clone();

    for y in 0..h{
      for x in 0..w{
        let neighbors = self.count_neighbors(&tiles, x, y, w, h);
        if let Some((tile_type, count)) = neighbors.iter().max_by_key(|(_, c)| *c){
          if *count >= 5{
            smoothed[y as usize][x as usize] = WildernessTile::new(*tile_type);
          }
        }
      }
    }

    smoothed
  }

  fn count_neighbors(&self,
    tiles: &[Vec<WildernessTile>],
    x: i32,
    y: i32,
    w: i32,
    h: i32
  ) -> Vec<(WildernessTileType, usize)>{
    let mut counts: HashMap<WildernessTileType, usize> = HashMap::new();

    for dy in -1..=1{
      for dx in -1..=1{
        if dx == 0 && dy == 0 {continue;}
        let nx = x + dx;
        let ny = y + dy;

        if nx >= 0 && nx < w && ny >= 0 && ny < h{
          let tile_type = tiles[ny as usize][nx as usize].tile_type;
          *counts.entry(tile_type).or_insert(0) += 1;
        }
      }
    }

    counts.into_iter().collect()
  }
}
