use std::io::{Stdout, Write};

use anyhow::Result;
use crossterm::{
  cursor, queue,
  style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
};

use crate::combat::picker::MonsterPicker;

pub fn draw(
  stdout: &mut Stdout,
  picker: &MonsterPicker,
  screen_width: u16,
  screen_height: u16,
) -> Result<()>{
  let width=screen_width.min(64);
  if width < 20{return Ok(())}
  let matches=picker.matches();
  let visible=matches.len().min(6) as u16;
  let height=visible + 7;
  let x=screen_width.saturating_sub(width)/2;
  let y=screen_height.saturating_sub(height)/2;
  let inner=width.saturating_sub(2) as usize;

  for row in 0..height{
    queue!(stdout,cursor::MoveTo(x,y+row),SetBackgroundColor(Color::DarkBlue),Print(" ".repeat(width as usize)))?;
  }
  line(stdout,x,y,width," SPAWN MONSTER ",Color::Cyan)?;
  text(stdout,x+2,y+2,&format!("Search: {}_",picker.query()),Color::White)?;
  text(stdout,x+2,y+3,&"─".repeat(inner.saturating_sub(2)),Color::DarkGrey)?;

  if matches.is_empty(){
    text(stdout,x+3,y+4,"No matching monsters",Color::DarkGrey)?;
  }else{
    for (index,choice) in matches.iter().take(6).enumerate(){
      let marker=if index==picker.selected_index(){">"}else{" "};
      let hd=choice.hit_dice.first().copied().unwrap_or(0);
      let content=format!("{marker} {:<24} HD {:>2}  AC {:>2}",choice.name,hd,choice.armor_class);
      text(
        stdout,x+2,y+4+index as u16,&truncate(&content,inner.saturating_sub(2)),
        if index==picker.selected_index(){Color::Yellow}else{Color::Grey},
      )?;
    }
  }
  text(stdout,x+2,y+height-2,"Type to filter · ↑/↓ select · Enter spawn · Esc cancel",Color::DarkGrey)?;
  queue!(stdout,ResetColor)?;
  Ok(())
}

fn line(stdout:&mut Stdout,x:u16,y:u16,width:u16,title:&str,color:Color)->Result<()>{
  let fill=width.saturating_sub(2) as usize;
  let label=truncate(title,fill);
  let remaining=fill.saturating_sub(label.len());
  text(stdout,x,y,&format!("┌{}{}┐",label,"─".repeat(remaining)),color)
}

fn text(stdout:&mut Stdout,x:u16,y:u16,value:&str,color:Color)->Result<()>{
  queue!(stdout,cursor::MoveTo(x,y),SetForegroundColor(color),SetBackgroundColor(Color::DarkBlue))?;
  write!(stdout,"{value}")?;
  Ok(())
}

fn truncate(value:&str,width:usize)->String{
  value.chars().take(width).collect()
}
