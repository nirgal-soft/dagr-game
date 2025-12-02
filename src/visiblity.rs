use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Visibility{
  #[default]
  Unseen,
  Seen,
  Visible,
}

#[derive(Clone, Copy)]
pub struct Slope{
  y: i32,
  x: i32,
}

impl Slope{
  pub fn new(y: i32, x: i32) -> Self{
    Slope{y, x}
  }

  pub fn greater(&self, y: i32, x: i32) -> bool{
    self.y * x > self.x * y
  }

  pub fn greater_or_equal(&self, y: i32, x: i32) -> bool{
    self.y * x >= self.x * y
  }

  fn less(&self, y: i32, x: i32) -> bool{
    self.y * x < self.x * y
  }
}

pub struct VisibilityMap{
  visible: HashSet<(i32, i32)>,
  seen: HashSet<(i32, i32)>,
  view_radius: i32,
}

impl VisibilityMap{
  pub fn new(view_radius: i32) -> Self{
    Self{
      visible: HashSet::new(),
      seen: HashSet::new(),
      view_radius,
    }
  }

  pub fn get(&self, x: i32, y: i32) -> Visibility{
    if self.visible.contains(&(x, y)){
      Visibility::Visible
    }else if self.seen.contains(&(x, y)){
      Visibility::Seen
    }else{
      Visibility::Unseen
    }
  }

  pub fn set_view_radius(&mut self, radius: i32){
    self.view_radius = radius;
  }

  pub fn clear(&mut self){
    self.visible.clear();
    self.seen.clear();
  }

  pub fn set_visible(&mut self, x: i32, y: i32){
    self.visible.insert((x, y));
    self.seen.insert((x, y));
  }

  pub fn update<F>(&mut self, origin_x: i32, origin_y: i32, is_opaque: F)
  where 
    F: Fn(i32, i32) -> bool,
  {
    self.visible.clear();

    self.compute_fov(origin_x, origin_y, &is_opaque)
  }

  fn compute_fov<F>(&mut self, origin_x: i32, origin_y: i32, is_opaque: &F)
  where
    F: Fn(i32, i32) -> bool,
  {
    self.set_visible(origin_x, origin_y);

    for octant in 0..8{
      self.compute_octant(
        octant,
        origin_x,
        origin_y,
        is_opaque,
        1,
        Slope::new(1, 1),
        Slope::new(0, 1),
      );
    }
  }

  fn compute_octant<F>(
    &mut self,
    octant: u8,
    origin_x: i32,
    origin_y: i32,
    is_opaque: &F,
    mut x: i32,
    mut top: Slope,
    mut bottom: Slope,
  ) where 
    F: Fn(i32, i32) -> bool,
  {
    while x <= self.view_radius{
      let top_y = if top.x == 1{
        x
      }else{
        ((x*2+1)*top.y+top.x-1)/(top.x*2)
      };

      let bottom_y = if bottom.y == 0{
        0
      }else{
        ((x*2-1)*bottom.y+bottom.x)/(bottom.x*2)
      };

      let mut was_opaque: i32 = -1;

      for y in (bottom_y..=top_y).rev(){
        let (tx, ty) = self.transform_octant(octant, origin_x, origin_y, x, y);

        let in_range = x*x+y*y <= self.view_radius * self.view_radius;

        if in_range{
          self.set_visible(tx, ty);
        }

        let tile_opaque = !in_range || is_opaque(tx, ty);

        if x != self.view_radius{
          if tile_opaque{
            if was_opaque == 0{
              let new_top = Slope::new(y*2+1, x*2-1);
              if !bottom.greater_or_equal(new_top.y, new_top.x){
                self.compute_octant(
                  octant,
                  origin_x,
                  origin_y,
                  is_opaque,
                  x+1,
                  new_top,
                  bottom,
                );
              }
            }
            was_opaque = 1;
          }else{
            if was_opaque == 1 && y > 0{
              bottom = Slope::new(y*2+1, x*2+1);
            }
            was_opaque = 0;
          }
        }
      }

      if was_opaque != 0{
        break;
      }

      x += 1;
    }
  }
 
  fn transform_octant(&self, octant: u8, ox: i32, oy: i32, x: i32, y: i32) -> (i32, i32){
    match octant{
      0 => (ox + x, oy - y),
      1 => (ox + y, oy - x),
      2 => (ox - y, oy - x),
      3 => (ox - x, oy - y),
      4 => (ox - x, oy + y),
      5 => (ox - y, oy + x),
      6 => (ox + y, oy + x),
      7 => (ox + x, oy + y),
      _ => (ox, oy),
    }
  }

  fn is_in_range(&self, x: i32, y: i32) -> bool{
    x * x + y * y <= self.view_radius * self.view_radius
  }
}
