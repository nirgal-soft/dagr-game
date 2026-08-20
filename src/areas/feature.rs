use crate::renderer::Tile;
use crossterm::style::Color;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Feature {
    pub tile: Tile,
    pub walkable: bool,
    pub opaque: bool,
}

impl Feature {
    pub const fn new(symbol: char, fg: Color, walkable: bool, opaque: bool) -> Self {
        Self {
            tile: Tile {
                symbol,
                fg,
                bg: Color::Black,
            },
            walkable,
            opaque,
        }
    }

    pub const fn with_bg(mut self, bg: Color) -> Self {
        self.tile.bg = bg;
        self
    }

    //common features, TODO abstract these later
    pub const WALL: Self = Self::new('#', Color::White, false, true).with_bg(Color::Rgb {
        r: 40,
        g: 35,
        b: 30,
    });
    pub const TREE: Self = Self::new('T', Color::DarkGreen, true, true);
    pub const BRUSH: Self = Self::new('"', Color::Green, true, false);
    pub const ROCK: Self = Self::new('o', Color::Grey, false, true);
    pub const RUBBLE: Self = Self::new(':', Color::DarkGrey, true, false);
    pub const WATER: Self = Self::new('~', Color::Blue, true, false);
    pub const LANDMARK: Self = Self::new('*', Color::Yellow, true, false);
    pub const RUINS: Self = Self::new('&', Color::DarkYellow, true, false);
    pub const CAVE: Self = Self::new('O', Color::DarkGrey, true, false);
    pub const TOMB: Self = Self::new('†', Color::Grey, true, false);
    pub const LAIR: Self = Self::new('!', Color::Red, true, false);
    pub const RESOURCE: Self = Self::new('$', Color::Green, true, false);
    pub const SETTLEMENT: Self = Self::new('H', Color::Yellow, true, false);
    pub const MINE: Self = Self::new('m', Color::Grey, true, false);
    pub const STAIRS_UP: Self = Self::new('<', Color::Yellow, true, false).with_bg(Color::Rgb {
        r: 50,
        g: 40,
        b: 20,
    });
    pub const STAIRS_DOWN: Self = Self::new('>', Color::Yellow, true, false).with_bg(Color::Rgb {
        r: 50,
        g: 40,
        b: 20,
    });
    pub const DOOR_CLOSED: Self = Self::new('+', Color::White, false, true);
    pub const DOOR_OPEN: Self = Self::new('/', Color::White, true, false);

    pub fn inspection(&self) -> (&'static str, &'static str) {
        match *self {
            Self::WALL => ("Stone wall", "A solid barrier of fitted or natural stone."),
            Self::TREE => ("Tree", "A mature tree with a dense, sight-blocking crown."),
            Self::BRUSH => ("Brush", "Low brush and tangled undergrowth."),
            Self::ROCK => ("Boulder", "A heavy outcrop that blocks passage."),
            Self::RUBBLE => ("Rubble", "Loose stones and broken ground."),
            Self::WATER => ("Water", "Open water shallow enough to cross cautiously."),
            Self::LANDMARK => ("Landmark", "A conspicuous feature worth investigating."),
            Self::RUINS => (
                "Ancient ruins",
                "Weathered remains from an earlier occupation.",
            ),
            Self::CAVE => ("Cave mouth", "A dark opening in the earth or stone."),
            Self::TOMB => ("Forgotten tomb", "A place built to hold the dead."),
            Self::LAIR => (
                "Signs of a lair",
                "Tracks and disturbance suggest something lives nearby.",
            ),
            Self::RESOURCE => (
                "Natural resource",
                "A useful concentration of local material.",
            ),
            Self::SETTLEMENT => (
                "Settlement",
                "A place where people have established a lasting presence.",
            ),
            Self::MINE => (
                "Mine workings",
                "An excavated site following material below the surface.",
            ),
            Self::STAIRS_UP => ("Way up", "A route to the level above."),
            Self::STAIRS_DOWN => ("Way down", "A route to the level below."),
            Self::DOOR_CLOSED => ("Closed door", "A closed barrier that may be opened."),
            Self::DOOR_OPEN => ("Open door", "An open passage through a doorway."),
            _ => ("Feature", "A visible feature of this place."),
        }
    }

    pub fn is_stairs_up(&self) -> bool {
        self.tile.symbol == '<'
    }

    pub fn is_stairs_down(&self) -> bool {
        self.tile.symbol == '>'
    }
}
