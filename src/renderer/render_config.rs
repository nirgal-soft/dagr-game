use super::tile::Tile;
use crossterm::style::Color;

#[derive(Clone, Debug)]
pub struct RenderConfig {
    pub unseen_symbol: char,
    pub unseen_fg: Color,
    pub unseen_bg: Color,
    pub player_symbol: char,
    pub player_fg: Color,
    pub player_bg: Color,
    pub dim_factor: f32,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            unseen_symbol: ' ',
            unseen_fg: Color::Black,
            unseen_bg: Color::Black,
            player_symbol: '@',
            player_fg: Color::Red,
            player_bg: Color::Black,
            dim_factor: 0.4,
        }
    }
}

impl RenderConfig {
    pub fn dim_color(&self, color: Color) -> Color {
        match color {
            Color::Black => Color::Black,
            Color::White => Color::DarkGrey,
            Color::Grey => Color::DarkGrey,
            Color::Yellow => Color::DarkYellow,
            Color::Green => Color::DarkGreen,
            Color::Blue => Color::DarkBlue,
            Color::Red => Color::DarkRed,
            Color::Cyan => Color::DarkCyan,
            Color::Magenta => Color::DarkMagenta,
            Color::DarkGrey => Color::Rgb {
                r: 30,
                g: 30,
                b: 30,
            },
            Color::DarkYellow => Color::Rgb { r: 60, g: 60, b: 0 },
            Color::DarkGreen => Color::Rgb { r: 0, g: 40, b: 0 },
            Color::DarkBlue => Color::Rgb { r: 0, g: 0, b: 40 },
            Color::DarkRed => Color::Rgb { r: 40, g: 0, b: 0 },
            Color::DarkCyan => Color::Rgb { r: 0, g: 40, b: 40 },
            Color::DarkMagenta => Color::Rgb { r: 40, g: 0, b: 40 },
            Color::Rgb { r, g, b } => Color::Rgb {
                r: (r as f32 * self.dim_factor) as u8,
                g: (g as f32 * self.dim_factor) as u8,
                b: (b as f32 * self.dim_factor) as u8,
            },
            other => other,
        }
    }

    pub fn player_tile(&self) -> Tile {
        Tile {
            symbol: self.player_symbol,
            fg: self.player_fg,
            bg: self.player_bg,
        }
    }
}
