use std::time::Duration;
use crossterm::event::{self, Event, KeyCode, KeyEvent};

pub enum Action{
  Move(i32, i32),
  Quit,
  Wait,
  None,
}

#[derive(Default, Debug)]
pub struct InputManager;

impl InputManager{
  pub fn new() -> Self{
    Self
  }

  pub fn poll_input(&self) -> Action{
    if event::poll(Duration::from_millis(0)).unwrap_or(false){
      if let Ok(Event::Key(key)) = event::read(){
        return self.key_to_action(key);
      }
    }

    Action::None
  }

  pub fn wait_for_input(&self) -> Action{
    loop{
      if let Ok(Event::Key(key)) = event::read(){
        let action = self.key_to_action(key);
        if !matches!(action, Action::None){
          return action;
        }
      }
    }
  }

  fn key_to_action(&self, key: KeyEvent) -> Action{
    match key.code{
      KeyCode::Up | KeyCode::Char('k') => Action::Move(0, -1),
      KeyCode::Down | KeyCode::Char('j') => Action::Move(0, 1),
      KeyCode::Left | KeyCode::Char('h') => Action::Move(-1, 0),
      KeyCode::Right | KeyCode::Char('l') => Action::Move(1, 0),
      KeyCode::Char('y') => Action::Move(-1, -1),
      KeyCode::Char('u') => Action::Move(1, -1),
      KeyCode::Char('b') => Action::Move(-1, 1),
      KeyCode::Char('n') => Action::Move(1, 1),

      KeyCode::Char('q') => Action::Quit,

      KeyCode::Char('.') => Action::Wait,

      _ => Action::None,
    }
  }
}
