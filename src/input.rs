use std::time::Duration;
use crossterm::event::{self, Event, KeyCode, KeyEvent};

pub enum Action{
  Move(i32, i32),
  Quit,
  Wait,
  Ascend,
  Descend,
  GenerateDungeon,
  OpenMonsterPicker,
  ResetArena,
  PickerConfirm,
  PickerCancel,
  PickerBackspace,
  PickerMove(i32),
  PickerInput(char),
  Dismiss,
  Explore,
  None,
}

#[derive(Default, Debug)]
pub struct InputManager;

impl InputManager{
  pub fn new() -> Self{
    Self
  }

  pub fn poll_input(&self, text_mode: bool) -> Action{
    if event::poll(Duration::from_millis(0)).unwrap_or(false){
      if let Ok(Event::Key(key)) = event::read(){
        return self.key_to_action(key, text_mode);
      }
    }

    Action::None
  }

  pub fn wait_for_input(&self) -> Action{
    loop{
      if let Ok(Event::Key(key)) = event::read(){
        let action = self.key_to_action(key, false);
        if !matches!(action, Action::None){
          return action;
        }
      }
    }
  }

  fn key_to_action(&self, key: KeyEvent, text_mode: bool) -> Action{
    if text_mode{
      return match key.code{
        KeyCode::Enter => Action::PickerConfirm,
        KeyCode::Esc => Action::PickerCancel,
        KeyCode::Backspace => Action::PickerBackspace,
        KeyCode::Up => Action::PickerMove(-1),
        KeyCode::Down => Action::PickerMove(1),
        KeyCode::Char(character) => Action::PickerInput(character),
        _ => Action::None,
      }
    }
    match key.code{
      KeyCode::Up | KeyCode::Char('k') => Action::Move(0, -1),
      KeyCode::Down | KeyCode::Char('j') => Action::Move(0, 1),
      KeyCode::Left | KeyCode::Char('h') => Action::Move(-1, 0),
      KeyCode::Right | KeyCode::Char('l') => Action::Move(1, 0),
      KeyCode::Char('y') => Action::Move(-1, -1),
      KeyCode::Char('u') => Action::Move(1, -1),
      KeyCode::Char('b') => Action::Move(-1, 1),
      KeyCode::Char('n') => Action::Move(1, 1),
      KeyCode::Char('<') => Action::Ascend,
      KeyCode::Char('>') => Action::Descend,
      KeyCode::Char(' ') => Action::Dismiss,
      KeyCode::Char('q') => Action::Quit,
      KeyCode::Char('.') => Action::Wait,
      KeyCode::Char('o') | KeyCode::Char('O') => Action::Explore,
      KeyCode::Char('D') => Action::GenerateDungeon,
      KeyCode::Char('M') => Action::OpenMonsterPicker,
      KeyCode::Char('R') => Action::ResetArena,

      _ => Action::None,
    }
  }
}

#[cfg(test)]
mod tests{
  use super::*;
  use crossterm::event::KeyModifiers;

  #[test]
  fn auto_explore_accepts_lowercase_and_uppercase_o(){
    let input = InputManager::new();
    for key in ['o', 'O']{
      let action = input.key_to_action(KeyEvent::new(KeyCode::Char(key), KeyModifiers::NONE), false);
      assert!(matches!(action, Action::Explore));
    }
  }
}
