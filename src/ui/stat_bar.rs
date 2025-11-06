use std::io::{self, Write};
use anyhow::Result;
use crossterm::{
  cursor, 
  queue, 
  style::Color, 
};
use super::bar::Bar;

pub struct StatBar{
  pub x: u16,
  pub y: u16,
  pub label: String,
  pub current: u32,
  pub max: u32,
  pub bar_width: u16
}

impl StatBar{
  pub fn new(x: u16, y: u16, label: String, current: u32, max: u32, bar_width: u16) -> Self{
    Self{
      x,
      y,
      label,
      current,
      max,
      bar_width,
    }
  }

  pub fn draw(&self, stdout: &mut io::Stdout) -> Result<()>{
    //raw the label
    queue!(stdout, cursor::MoveTo(self.x, self.y))?;
    write!(stdout, "{}: ", self.label)?;

    let label_len = self.label.len() + 2;
    let bar_x = self.x + label_len as u16;

    //draw the bar
    let bar = Bar::new(
      bar_x, self.y,
      self.bar_width,
      self.current,
      self.max,
      Color::Green
    );
    bar.draw(stdout)?;

    //draw the value
    let value_text = format!("{}/{}", self.current, self.max);
    let value_x = bar_x + self.bar_width + 1;
    queue!(stdout, cursor::MoveTo(value_x, self.y))?;
    write!(stdout, "{}", value_text)?;

    Ok(())
  }
}
