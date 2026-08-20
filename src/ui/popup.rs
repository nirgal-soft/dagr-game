use anyhow::Result;
use crossterm::{
    cursor, queue,
    style::{self, Color, Stylize},
};
use std::io::Stdout;

pub struct Popup {
    pub message: String,
    pub screen_width: u16,
    pub screen_height: u16,
}

impl Popup {
    pub fn new(message: String, screen_width: u16, screen_height: u16) -> Self {
        Self {
            message,
            screen_width,
            screen_height,
        }
    }

    pub fn draw(&self, stdout: &mut Stdout) -> Result<()> {
        let padding = 4;
        let msg_len = self.message.len() as u16;
        let box_width = msg_len + padding;
        let box_height = 5;

        let x = (self.screen_width.saturating_sub(box_width)) / 2;
        let y = (self.screen_height.saturating_sub(box_height)) / 2;

        for row in 0..box_height {
            queue!(stdout, cursor::MoveTo(x, y + row))?;
            let line = " ".repeat(box_width as usize);
            queue!(stdout, style::PrintStyledContent(line.on(Color::DarkBlue)))?;
        }

        queue!(stdout, cursor::MoveTo(x, y))?;
        let top = format!("┌{}┐", "─".repeat((box_width - 2) as usize));
        queue!(
            stdout,
            style::PrintStyledContent(top.with(Color::White).on(Color::DarkBlue))
        )?;

        queue!(stdout, cursor::MoveTo(x, y + 1))?;
        let side = format!("│{}│", " ".repeat((box_width - 2) as usize));
        queue!(
            stdout,
            style::PrintStyledContent(side.with(Color::White).on(Color::DarkBlue))
        )?;

        queue!(stdout, cursor::MoveTo(x, y + 2))?;
        let msg_padding = (box_width - 2 - msg_len) / 2;
        let msg_line = format!(
            "│{}{}{}│",
            " ".repeat(msg_padding as usize),
            &self.message,
            " ".repeat((box_width - 2 - msg_len - msg_padding) as usize),
        );
        queue!(
            stdout,
            style::PrintStyledContent(msg_line.with(Color::Yellow).on(Color::DarkBlue))
        )?;

        queue!(stdout, cursor::MoveTo(x, y + 3))?;
        let hint = "[Space] to dismiss";
        let hint_padding = (box_width - 2 - hint.len() as u16) / 2;
        let hint_line = format!(
            "│{}{}{}│",
            " ".repeat(hint_padding as usize),
            hint,
            " ".repeat((box_width - 2 - hint.len() as u16 - hint_padding) as usize),
        );
        queue!(
            stdout,
            style::PrintStyledContent(hint_line.with(Color::Grey).on(Color::DarkBlue))
        )?;

        queue!(stdout, cursor::MoveTo(x, y + 4))?;
        let bottom = format!("└{}┘", "─".repeat((box_width - 2) as usize));
        queue!(
            stdout,
            style::PrintStyledContent(bottom.with(Color::White).on(Color::DarkBlue))
        )?;

        Ok(())
    }
}
