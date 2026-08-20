use crossterm::style::Color;

use super::Pos;
use crate::renderer::Tile;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FixtureKind {
    Bedroll,
    FirePit,
    Tracks,
    Bones,
    Refuse,
}

impl FixtureKind {
    pub fn tile(self) -> Tile {
        match self {
            Self::Bedroll => Tile::new('=', Color::DarkYellow),
            Self::FirePit => Tile::new('*', Color::Red),
            Self::Tracks => Tile::new(';', Color::DarkGrey),
            Self::Bones => Tile::new('%', Color::Grey),
            Self::Refuse => Tile::new(',', Color::DarkYellow),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Fixture {
    pub pos: Pos,
    pub kind: FixtureKind,
    pub name: String,
    pub description: String,
}

impl Fixture {
    pub fn new(
        pos: Pos,
        kind: FixtureKind,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            pos,
            kind,
            name: name.into(),
            description: description.into(),
        }
    }
}
