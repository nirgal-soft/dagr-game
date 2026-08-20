use super::{border_style::BorderStyle, draw_box};
use crate::renderer::Tile;
use anyhow::Result;
use crossterm::{
    cursor, queue, style,
    style::{Color, ResetColor, SetBackgroundColor, SetForegroundColor},
};
use std::io;

pub struct Map {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

impl Map {
    pub fn new(x: u16, y: u16, w: u16, h: u16) -> Self {
        Self { x, y, w, h }
    }

    pub fn draw_cell(
        &self,
        stdout: &mut io::Stdout,
        screen_x: u16,
        screen_y: u16,
        tile: Option<Tile>,
    ) -> Result<()> {
        if screen_x >= self.w.saturating_sub(2) || screen_y >= self.h.saturating_sub(2) {
            return Ok(());
        }
        queue!(
            stdout,
            cursor::MoveTo(self.x + screen_x + 1, self.y + screen_y + 1)
        )?;
        if let Some(tile) = tile {
            queue!(
                stdout,
                SetForegroundColor(tile.fg),
                SetBackgroundColor(tile.bg),
                style::Print(tile.symbol)
            )?;
        } else {
            queue!(
                stdout,
                SetForegroundColor(Color::Black),
                SetBackgroundColor(Color::Black),
                style::Print(' ')
            )?;
        }
        queue!(stdout, ResetColor)?;
        Ok(())
    }

    pub fn draw<F>(&self, stdout: &mut io::Stdout, mut tile_fn: F) -> Result<()>
    where
        F: FnMut(u16, u16) -> Option<Tile>,
    {
        if self.w < 2 || self.h < 2 {
            return Ok(());
        }

        draw_box(stdout, self.x, self.y, self.w, self.h, BorderStyle::SINGLE)?;

        for y in 1..self.h - 1 {
            for x in 1..self.w - 1 {
                queue!(stdout, cursor::MoveTo(self.x + x, self.y + y))?;

                if let Some(tile) = tile_fn(x - 1, y - 1) {
                    queue!(
                        stdout,
                        SetForegroundColor(tile.fg),
                        SetBackgroundColor(tile.bg),
                        style::Print(tile.symbol),
                    )?;
                } else {
                    queue!(
                        stdout,
                        SetForegroundColor(Color::Black),
                        SetBackgroundColor(Color::Black),
                        style::Print(' '),
                    )?;
                }
            }
        }

        queue!(stdout, ResetColor)?;

        Ok(())
    }
}
