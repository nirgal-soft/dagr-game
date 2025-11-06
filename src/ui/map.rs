use std::io::{self, Write};
use anyhow::Result;
use crossterm::{
  cursor, 
  queue, 
  style::{
    Color, 
    ResetColor,
    SetForegroundColor,
  },
};
use super::{draw_box, border_style::BorderStyle};

pub struct Map{
  pub x: u16,
  pub y: u16,
  pub w: u16,
  pub h: u16,
}

impl Map{
  pub fn new(x: u16, y: u16, w: u16, h: u16) -> Self{
    Self{
      x,
      y,
      w,
      h,
    }
  }

  pub fn draw<F>(&self, stdout: &mut io::Stdout, mut tile_fn: F) -> Result<()>
  where 
    F: FnMut(u16, u16) -> Option<(char, Color)>
  {
    draw_box(stdout, self.x, self.y, self.w, self.h, BorderStyle::SINGLE)?;

    for y in 1..self.h-1{
      for x in 1..self.w-1{
        queue!(stdout, cursor::MoveTo(self.x+x, self.y+y))?;

        if let Some((symbol, color)) = tile_fn(x-1, y-1){
          queue!(stdout, SetForegroundColor(color))?;
          write!(stdout, "{}", symbol)?;
          queue!(stdout, ResetColor)?;
        }else{
          write!(stdout, " ")?;
        }
      }
    }

    Ok(())
  }
}
