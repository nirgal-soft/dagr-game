use crate::areas::Pos;

use super::{
  command::{ConsoleCommand, parse},
  hex_editor::HexEditor,
};

#[derive(Default)]
pub struct DebugSession {
  enabled: bool,
  console: Option<String>,
  editor: Option<HexEditor>,
  message: Option<String>,
}

impl DebugSession {
  pub fn enabled(&self) -> bool {
    self.enabled
  }
  pub fn console(&self) -> Option<&str> {
    self.console.as_deref()
  }
  pub fn console_open(&self) -> bool {
    self.console.is_some()
  }
  pub fn editor(&self) -> Option<&HexEditor> {
    self.editor.as_ref()
  }
  pub fn editor_mut(&mut self) -> Option<&mut HexEditor> {
    self.editor.as_mut()
  }
  pub fn editor_open(&self) -> bool {
    self.editor.is_some()
  }
  pub fn message(&self) -> Option<&str> {
    self.message.as_deref()
  }

  pub fn open_console(&mut self) {
    self.console = Some(String::new());
    self.message = None;
  }
  pub fn close_console(&mut self) {
    self.console = None;
  }
  pub fn input(&mut self, ch: char) {
    if let Some(input) = self.console.as_mut() {
      input.push(ch)
    }
  }
  pub fn backspace(&mut self) {
    if let Some(input) = self.console.as_mut() {
      input.pop();
    }
  }
  pub fn execute(&mut self) {
    let input = self.console.take().unwrap_or_default();
    self.message = Some(match parse(&input) {
      Ok(ConsoleCommand::DebugOn) => {
        self.enabled = true;
        "Debug mode enabled".into()
      }
      Ok(ConsoleCommand::DebugOff) => {
        self.enabled = false;
        self.editor = None;
        "Debug mode disabled".into()
      }
      Ok(ConsoleCommand::DebugStatus) => {
        format!("Debug mode is {}", if self.enabled { "on" } else { "off" })
      }
      Ok(ConsoleCommand::Help) => "Commands: debug on | debug off | debug status | help".into(),
      Err(error) => error,
    });
  }
  pub fn open_editor(&mut self, cursor: Pos) -> bool {
    if !self.enabled {
      return false;
    }
    self.editor = Some(HexEditor::new(cursor));
    true
  }
  pub fn close_editor(&mut self) {
    self.editor = None;
  }
  pub fn clear_message(&mut self) {
    self.message = None;
  }
  pub fn set_message(&mut self, message: impl Into<String>) {
    self.message = Some(message.into());
  }
}
