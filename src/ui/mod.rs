pub mod bar;
pub mod border_style;
pub mod map;
pub mod panel;
pub mod stat_bar;

use std::io::{self, Write};
use anyhow::Result;
use crossterm::{
  cursor, 
  queue, 
  style::{
    Color, 
    ResetColor,
    SetForegroundColor,
    SetBackgroundColor,
  },
};
use border_style::BorderStyle;

pub fn draw_box(stdout: &mut io::Stdout, x: u16, y: u16, w: u16, h: u16, style: BorderStyle) -> Result<()>{
  //top
  queue!(stdout, cursor::MoveTo(x, y))?;
  write!(stdout, "{}", style.top_left)?;
  write!(stdout, "{}", style.horizontal.to_string().repeat(w as usize - 2))?;
  write!(stdout, "{}", style.top_right)?;

  //sides
  for row in 1..h - 1{
    queue!(stdout, cursor::MoveTo(x, y + row))?;
    write!(stdout, "{}", style.vertical)?;
    queue!(stdout, cursor::MoveTo(x + w - 1, y + row))?;
    write!(stdout, "{}", style.vertical)?;
  }

  //bottom
  queue!(stdout, cursor::MoveTo(x, y + h - 1))?;
  write!(stdout, "{}", style.bottom_left)?;
  write!(stdout, "{}", style.horizontal.to_string().repeat(w as usize - 2))?;
  write!(stdout, "{}", style.bottom_right)?;

  Ok(())
}

pub fn draw_text(stdout: &mut io::Stdout, x: u16, y: u16, text: &str, fg: Color, bg: Color) -> Result<()>{
  queue!(
    stdout,
    cursor::MoveTo(x, y),
    SetForegroundColor(fg),
    SetBackgroundColor(bg),
  )?;

  write!(stdout, "{}", text)?;
  queue!(stdout, ResetColor)?;

  Ok(())
}

pub fn clear_region(stdout: &mut io::Stdout, x: u16, y: u16, w: u16, h: u16) -> Result<()>{
  let blank = " ".repeat(w as usize);
  
  for row in 0..h{
    queue!(stdout, cursor::MoveTo(x, y + row))?;
    write!(stdout, "{}", blank)?;
  }

  Ok(())
}
