use std::io::{self, Write};

use anyhow::Result;
use crossterm::{
  cursor, queue,
  style::{Attribute, Color, ResetColor, SetAttribute, SetForegroundColor},
};

pub struct StatBar{
  pub x: u16,
  pub y: u16,
  pub label: String,
  pub current: u32,
  pub max: u32,
  pub bar_width: u16,
}

impl StatBar{
  pub fn new(x:u16,y:u16,label:String,current:u32,max:u32,bar_width:u16)->Self{
    Self{x,y,label,current,max,bar_width}
  }

  pub fn draw(&self,stdout:&mut io::Stdout)->Result<()>{
    let ratio=if self.max==0{0.0}else{self.current as f32/self.max as f32};
    let filled=(ratio*self.bar_width as f32).round() as u16;
    let color=if ratio > 0.5{Color::Green}else if ratio > 0.25{Color::Yellow}else{Color::Red};
    let value=if self.max==0{"-- / --".to_string()}else{format!("{} / {}",self.current,self.max)};
    let width=(self.bar_width+12).max(30) as usize;
    let title=format!(" ♥ {} ",self.label);

    queue!(stdout,cursor::MoveTo(self.x,self.y),SetForegroundColor(Color::DarkGrey))?;
    write!(stdout,"╭{}{}╮",title,"─".repeat(width.saturating_sub(title.chars().count())))?;

    queue!(stdout,cursor::MoveTo(self.x,self.y+1),SetForegroundColor(Color::DarkGrey))?;
    write!(stdout,"│ [")?;
    queue!(stdout,SetForegroundColor(color),SetAttribute(Attribute::Bold))?;
    write!(stdout,"{}","█".repeat(filled as usize))?;
    queue!(stdout,SetForegroundColor(Color::DarkGrey),SetAttribute(Attribute::Reset))?;
    write!(stdout,"{}","·".repeat(self.bar_width.saturating_sub(filled) as usize))?;
    write!(stdout,"] {:>7} │",value)?;

    queue!(stdout,cursor::MoveTo(self.x,self.y+2),SetForegroundColor(Color::DarkGrey))?;
    write!(stdout,"╰{}╯","─".repeat(width))?;
    queue!(stdout,ResetColor)?;
    Ok(())
  }
}
