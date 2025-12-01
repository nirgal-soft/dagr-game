use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Visibility{
  #[default]
  Unseen,
  Seen,
  Visible,
}

pub struct VisibilityMap{
  tiles: HashMap<(i32, i32), Visibility>,
  view_radius: i32,
}

impl VisibilityMap{
  pub fn new(view_radius: i32) -> Self{
    Self{
      tiles: HashMap::new(),
      view_radius,
    }
  }

  pub fn get(&self, x: i32, y: i32) -> Visibility{
    self.tiles.get(&(x, y)).copied().unwrap_or(Visibility::Unseen)
  }

  pub fn set_view_radius(&mut self, radius: i32){
    self.view_radius = radius;
  }

  pub fn clear(&mut self){
    self.tiles.clear();
  }

  pub fn update<F>(&mut self, player_x: i32, player_y: i32, is_opaque: F)
  where 
    F: Fn(i32, i32) -> bool,
  {
    for visibility in self.tiles.values_mut(){
      if *visibility == Visibility::Visible{
        *visibility = Visibility::Seen;
      }
    }

    let visible_tiles = self.compute_fov(player_x, player_y, &is_opaque);

    for(x, y) in visible_tiles{
      self.tiles.insert((x, y), Visibility::Visible);
    }
  }

  fn compute_fov<F>(&self, origin_x: i32, origin_y: i32, is_opaque: &F) -> HashSet<(i32, i32)>
  where
    F: Fn(i32, i32) -> bool,
  {
    let mut visible = HashSet::new();
    visible.insert((origin_x, origin_y));

    for octant in 0..8{
      self.cast_light(
        &mut visible,
        origin_x,
        origin_y,
        1,
        1.0,
        0.0,
        octant,
        is_opaque,
      );
    }

    visible
  }

  fn cast_light<F>(
    &self,
    visible: &mut HashSet<(i32, i32)>,
    origin_x: i32,
    origin_y: i32,
    row: i32,
    mut start_slope: f32,
    end_slope: f32,
    octant: u8,
    is_opaque: &F,
  ) where
    F: Fn(i32, i32) -> bool,
  {
    if start_slope < end_slope || row > self.view_radius{
      return;
    }

    let mut prev_blocked = false;
    let mut saved_start_slope = start_slope;

    for col in -row..=0{
      let col = -col;

      let (dx, dy) = self.transform_octant(col, row, octant);
      let world_x = origin_x + dx;
      let world_y = origin_y + dy;

      let left_slope = (col as f32 - 0.5) / (row as f32 + 0.5);
      let right_slope = (col as f32 + 0.5) / (row as f32 - 0.5);

      if right_slope > start_slope{
        continue;
      }
      if left_slope < end_slope{
        break;
      }

      let distance_sq = dx * dx + dy * dy;
      if distance_sq <= self.view_radius * self.view_radius{
        visible.insert((world_x, world_y));
      }

      let blocked = is_opaque(world_x, world_y);

      if prev_blocked{
        if blocked{
          saved_start_slope = right_slope;
        }else{
          prev_blocked = false;
          start_slope = saved_start_slope;
        }
      }else if blocked && row < self.view_radius{
        prev_blocked = true;
        self.cast_light(
          visible,
          origin_x,
          origin_y,
          row+1,
          start_slope,
          left_slope,
          octant,
          is_opaque,
        );
        saved_start_slope = right_slope;
      }
    }

    if !prev_blocked{
      self.cast_light(
        visible,
        origin_x,
        origin_y,
        row+1,
        start_slope,
        end_slope,
        octant,
        is_opaque,
      );
    }
  }

  fn transform_octant(&self, col: i32, row: i32, octant: u8) -> (i32, i32){
    match octant{
      0 => (col, -row),
      1 => (row, -col),
      2 => (row, col),
      3 => (col, row),
      4 => (-col, row),
      5 => (-row, col),
      6 => (-row, -col),
      7 => (-col, -row),
      _ => (col, row),
    }
  }
}
