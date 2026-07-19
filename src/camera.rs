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

  pub fn pan_when_border_reached(&mut self, world_x:i32, world_y:i32){
    self.x=pan_axis(self.x,self.view_w,world_x);
    self.y=pan_axis(self.y,self.view_h,world_y);
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

fn pan_axis(origin:i32,extent:i32,target:i32)->i32{
  if extent<=2{return origin}
  if target<=origin || target>=origin+extent-1{
    target-(extent/2)
  }else{
    origin
  }
}

#[cfg(test)]
mod tests{
  use super::*;

  #[test]
  fn look_panning_waits_for_viewport_border(){
    let mut camera=Camera::new(10,8);
    camera.pan_when_border_reached(5,4);
    assert_eq!((camera.x,camera.y),(0,0));
    camera.pan_when_border_reached(9,4);
    assert_eq!((camera.x,camera.y),(4,0));
    camera.pan_when_border_reached(4,4);
    assert_eq!((camera.x,camera.y),(-1,0));
  }
}
