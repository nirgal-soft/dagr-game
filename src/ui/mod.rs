pub mod border_style;

use std::io::{self, Write};
use anyhow::{Context, Result};
use crossterm::{*, cursor, style::*};
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

  stdout.flush()
    .context("Failed to flush stdout after drawing box")?;

  Ok(())
}
