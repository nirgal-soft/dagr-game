pub struct Camera{
  pub x: i32,
  pub y: i32,
  view_w: i32,
  view_h: i32,
}

impl Camera{
  pub fn new(vw: u16, vh: u16) -> Self{
    Self {
      x: 0,
      y: 0,
      view_w: vw as i32,
      view_h: vh as i32
    }
  }

  pub fn center_on(&mut self, world_x: i32, world_y: i32){
    self.x = world_x - (self.view_w/2);
    self.y = world_y - (self.view_h/2);
  }

  pub fn world_to_screen(&self, world_x: i32, world_y: i32) -> Option<(u16, u16)>{
    let screen_x = world_x - self.x;
    let screen_y = world_y - self.y;
    if screen_x >= 0 && screen_x < self.view_w
      && screen_y >= 0 && screen_y < self.view_h{
      Some((screen_x as u16, screen_y as u16))
    }else{
      None
    }
  }
}
