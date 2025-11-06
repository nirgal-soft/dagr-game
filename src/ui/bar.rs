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

pub struct Bar{
  pub x: u16,
  pub y: u16,
  pub w: u16,
  pub current: u32,
  pub max: u32,
  pub full_char: char,
  pub empty_char: char,
  pub fg: Color,
}

impl Bar{
  pub fn new(x: u16,
    y: u16,
    w: u16,
    current: u32,
    max: u32,
    fg: Color
  ) -> Self{
    Self{
      x,
      y,
      w,
      current,
      max,
      full_char: '█',
      empty_char: '░',
      fg,
    }
  }

  pub fn draw(&self, stdout: &mut io::Stdout) -> Result<()>{
    let filled = if self.max > 0{
      ((self.current as f32 / self.max as f32) * self.w as f32).round() as u16
    }else{
      0
    };
    let empty = self.w.saturating_sub(filled);

    queue!(stdout, cursor::MoveTo(self.x, self.y), SetForegroundColor(self.fg))?;

    write!(stdout, "{}", self.full_char.to_string().repeat(filled as usize))?;
    write!(stdout, "{}", self.empty_char.to_string().repeat(empty as usize))?;

    queue!(stdout, ResetColor)?;

    Ok(())
  }
}
