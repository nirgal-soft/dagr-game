use std::time::Duration;
use crossterm::event::{self, Event, KeyCode, KeyEvent};

#[derive(Clone,Copy,Debug,Eq,PartialEq)]
pub enum InputMode{Normal,Text,HexEditor}

pub enum Action{
  Move(i32, i32),
  Quit,
  Wait,
  Ascend,
  Descend,
  GenerateDungeon,
  OpenMonsterPicker,
  ResetArena,
  ModalConfirm,
  ModalCancel,
  ModalBackspace,
  ModalMove(i32),
  ModalInput(char),
  Dismiss,
  Explore,
  Look,
  OpenConsole,
  EditorField(i32),
  EditorCycle(i32),
  PlaceHex,
  Cancel,
  None,
}

#[derive(Default, Debug)]
pub struct InputManager;

impl InputManager{
  pub fn new() -> Self{
    Self
  }

  pub fn poll_input(&self, mode: InputMode) -> Action{
    if event::poll(Duration::from_millis(0)).unwrap_or(false){
      if let Ok(Event::Key(key)) = event::read(){
        return self.key_to_action(key, mode);
      }
    }

    Action::None
  }

  pub fn wait_for_input(&self) -> Action{
    loop{
      if let Ok(Event::Key(key)) = event::read(){
        let action = self.key_to_action(key, InputMode::Normal);
        if !matches!(action, Action::None){
          return action;
        }
      }
    }
  }

  fn key_to_action(&self, key: KeyEvent, mode: InputMode) -> Action{
    if mode==InputMode::Text{
      return match key.code{
        KeyCode::Enter => Action::ModalConfirm,
        KeyCode::Esc => Action::ModalCancel,
        KeyCode::Backspace => Action::ModalBackspace,
        KeyCode::Up => Action::ModalMove(-1),
        KeyCode::Down => Action::ModalMove(1),
        KeyCode::Char(character) => Action::ModalInput(character),
        _ => Action::None,
      }
    }
    if mode==InputMode::HexEditor{
      return match key.code{
        KeyCode::Up=>Action::EditorField(-1),
        KeyCode::Down=>Action::EditorField(1),
        KeyCode::Left=>Action::EditorCycle(-1),
        KeyCode::Right=>Action::EditorCycle(1),
        KeyCode::Char('x')=>Action::PlaceHex,
        KeyCode::Esc=>Action::Cancel,
        KeyCode::Char('h')=>Action::Move(-1,0),
        KeyCode::Char('j')=>Action::Move(0,1),
        KeyCode::Char('k')=>Action::Move(0,-1),
        KeyCode::Char('l')=>Action::Move(1,0),
        KeyCode::Char('y')=>Action::Move(-1,-1),
        KeyCode::Char('u')=>Action::Move(1,-1),
        KeyCode::Char('b')=>Action::Move(-1,1),
        KeyCode::Char('n')=>Action::Move(1,1),
        _=>Action::None,
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
      KeyCode::Char('x') => Action::Look,
      KeyCode::Char(':') => Action::OpenConsole,
      KeyCode::Esc => Action::Cancel,
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
  fn x_enters_look_mode(){
    let input=InputManager::new();
    let action=input.key_to_action(KeyEvent::new(KeyCode::Char('x'),KeyModifiers::NONE),InputMode::Normal);
    assert!(matches!(action,Action::Look));
  }

  #[test]
  fn auto_explore_accepts_lowercase_and_uppercase_o(){
    let input = InputManager::new();
    for key in ['o', 'O']{
      let action = input.key_to_action(KeyEvent::new(KeyCode::Char(key), KeyModifiers::NONE), InputMode::Normal);
      assert!(matches!(action, Action::Explore));
    }
  }
}
