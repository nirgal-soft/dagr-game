use std::io::{Stdout, Write};

use anyhow::Result;
use crossterm::{
    cursor, queue,
    style::{Color, ResetColor, SetBackgroundColor, SetForegroundColor},
};

pub fn draw(stdout: &mut Stdout, input: &str, width: u16, height: u16) -> Result<()> {
    let value = format!(":{}▌", input);
    queue!(
        stdout,
        cursor::MoveTo(0, height.saturating_sub(1)),
        SetForegroundColor(Color::White),
        SetBackgroundColor(Color::DarkBlue),
    )?;
    write!(stdout, "{:<width$}", value, width = width as usize)?;
    queue!(stdout, ResetColor)?;
    Ok(())
}
