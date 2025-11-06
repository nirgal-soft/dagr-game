use std::io::{self, Write};
use anyhow::Result;
use crossterm::{
  cursor, 
  queue, 
  style::{
    Attribute,
    SetAttribute,
  },
};
use super::{draw_box, border_style::BorderStyle};

pub struct Panel{
  pub x: u16,
  pub y: u16,
  pub w: u16,
  pub h: u16,
  pub title: Option<String>,
  content: Vec<String>,
}

impl Panel{
  pub fn new(x: u16, y: u16, w: u16, h: u16) -> Self{
    Self{
      x,
      y,
      w,
      h,
      title: None,
      content: Vec::new(),
    }
  }

  pub fn set_title(&mut self, title: String){
    self.title = Some(title);
  }

  pub fn set_content(&mut self, content: Vec<String>){
    self.content = content;
  }

  pub fn add_content(&mut self, content: String){
    self.content.push(content);
  }

  pub fn draw(&self, stdout: &mut io::Stdout) -> Result<()>{
    draw_box(stdout, self.x, self.y, self.w, self.h, BorderStyle::SINGLE)?;
    if let Some(title_text) = &self.title{
      let title_x = self.x + 2;
      queue!(stdout, cursor::MoveTo(title_x, self.y), SetAttribute(Attribute::Bold))?;
      write!(stdout, " {} ", title_text)?;
      queue!(stdout, SetAttribute(Attribute::Reset))?;
    }

    let content_area_height = self.h.saturating_sub(2) as usize;
    let content_area_width = self.w.saturating_sub(2) as usize;

    for (i, line) in self.content.iter().take(content_area_height).enumerate(){
      let content_y = self.y + 1 + i as u16;
      let content_x = self.x + 1;

      let display_line = if line.len() > content_area_width{
        &line[..content_area_width]
      }else{
        line
      };

      queue!(stdout, cursor::MoveTo(content_x, content_y))?;
      write!(stdout, "{}", display_line)?;
    }

    Ok(())
  }
}
