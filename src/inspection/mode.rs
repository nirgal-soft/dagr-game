use crate::areas::Pos;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LookMode {
    cursor: Pos,
}

impl LookMode {
    pub fn new(cursor: Pos) -> Self {
        Self { cursor }
    }
    pub fn cursor(self) -> Pos {
        self.cursor
    }
    pub fn move_by(&mut self, dx: i32, dy: i32, bounds: (i32, i32)) {
        self.cursor.0 = (self.cursor.0 + dx).clamp(0, bounds.0.saturating_sub(1));
        self.cursor.1 = (self.cursor.1 + dy).clamp(0, bounds.1.saturating_sub(1));
    }
}
