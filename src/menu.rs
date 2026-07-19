use std::io::{self, Write};

use anyhow::Result;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MainMenuChoice{
  Play,
  DebugTools,
  Quit,
}

pub fn show_main_menu() -> Result<MainMenuChoice>{
  loop{
    print!("\x1b[2J\x1b[H");
    println!("DAGR");
    println!("====");
    println!();
    println!("1. Play roguelike client");
    println!("2. Debug engine tools");
    println!("q. Quit");
    println!();
    let input = prompt("Select: ")?;
    match input.trim().to_ascii_lowercase().as_str(){
      "1" | "p" | "play" => return Ok(MainMenuChoice::Play),
      "2" | "d" | "debug" => return Ok(MainMenuChoice::DebugTools),
      "q" | "quit" | "exit" => return Ok(MainMenuChoice::Quit),
      _ => {
        println!("Unknown selection. Press Enter to continue.");
        let _ = prompt("")?;
      }
    }
  }
}

pub fn prompt(label: &str) -> Result<String>{
  print!("{label}");
  io::stdout().flush()?;
  let mut input = String::new();
  io::stdin().read_line(&mut input)?;
  Ok(input.trim_end().to_string())
}

#[cfg(test)]
mod tests{
  use super::*;

  #[test]
  fn menu_choices_are_distinct(){
    assert_ne!(MainMenuChoice::Play, MainMenuChoice::DebugTools);
    assert_ne!(MainMenuChoice::DebugTools, MainMenuChoice::Quit);
  }
}
