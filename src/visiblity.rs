use std::collections::HashSet;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Visibility{
  #[default]
  Unseen,
  Seen,
  Visible,
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

  pub fn is_visible(&self, x: i32, y: i32) -> bool{
    self.visible.contains(&(x, y))
  }

  pub fn set_view_radius(&mut self, radius: i32){
    self.view_radius = radius;
  }

  pub fn clear(&mut self){
    self.visible.clear();
    self.seen.clear();
  }

  fn set_visible(&mut self, x: i32, y: i32){
    self.visible.insert((x, y));
    self.seen.insert((x, y));
  }

  pub fn visible_tiles(&self) -> impl Iterator<Item = (i32, i32)> + '_{
    self.visible.iter().copied()
  }

  pub fn update<F>(&mut self, origin_x: i32, origin_y: i32, is_opaque: F)
  where 
    F: Fn(i32, i32) -> bool,
  {
    self.visible.clear();
    self.set_visible(origin_x, origin_y);

    let r = self.view_radius;
    
    // Cast rays to every point on the perimeter of view square
    // (using more points gives better coverage)
    for i in -r..=r{
      // Cast to all four edges
      self.cast_ray(origin_x, origin_y, origin_x + r, origin_y + i, &is_opaque);
      self.cast_ray(origin_x, origin_y, origin_x - r, origin_y + i, &is_opaque);
      self.cast_ray(origin_x, origin_y, origin_x + i, origin_y + r, &is_opaque);
      self.cast_ray(origin_x, origin_y, origin_x + i, origin_y - r, &is_opaque);
    }
  }

  fn cast_ray<F>(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, is_opaque: &F)
  where
    F: Fn(i32, i32) -> bool,
  {
    // Bresenham's line algorithm
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1{ 1 }else{ -1 };
    let sy = if y0 < y1{ 1 }else{ -1 };
    let mut err = dx + dy;
    
    let mut x = x0;
    let mut y = y0;
    
    let r_sq = self.view_radius * self.view_radius;
    
    loop{
      // Check if within circular radius
      let dist_x = x - x0;
      let dist_y = y - y0;
      if dist_x * dist_x + dist_y * dist_y > r_sq{
        break;
      }
      
      self.set_visible(x, y);
      
      // Stop AFTER marking opaque tile visible (we can see the wall, not through it)
      if is_opaque(x, y) && (x != x0 || y != y0){
        break;
      }
      
      if x == x1 && y == y1{
        break;
      }
      
      let e2 = 2 * err;
      if e2 >= dy{
        err += dy;
        x += sx;
      }
      if e2 <= dx{
        err += dx;
        y += sy;
      }
    }
  }
}
